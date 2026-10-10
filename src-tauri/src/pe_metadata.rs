//! PE resource data is parsed from bounded bytes; no loader or binary runs.
use serde::{Deserialize,Serialize};
const BAD:&str="PE_VERSION_RESOURCE_INVALID";
pub const MAX_PE_BYTES:usize=64*1024*1024;
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeMetadata {
    pub product_version:Option<String>,pub file_version:Option<String>,
    pub product_name:Option<String>,pub company_name:Option<String>,
}
fn span(bytes:&[u8],offset:usize,length:usize)->Result<&[u8],&'static str>{
    bytes.get(offset..offset.checked_add(length).ok_or(BAD)?).ok_or(BAD)
}
fn word(bytes:&[u8],offset:usize)->Result<usize,&'static str>{
    let s=span(bytes,offset,2)?;Ok(u16::from_le_bytes([s[0],s[1]]) as usize)
}
fn dword(bytes:&[u8],offset:usize)->Result<usize,&'static str>{
    let s=span(bytes,offset,4)?;Ok(u32::from_le_bytes([s[0],s[1],s[2],s[3]]) as usize)
}
fn aligned(offset:usize)->Result<usize,&'static str>{Ok(offset.checked_add(3).ok_or(BAD)?&!3)}
fn map_rva(bytes:&[u8],sections:&[u8],rva:usize,length:usize)->Result<usize,&'static str>{
    let mut result=None;
    for section in sections.chunks_exact(40){
        let address=dword(section,12)?;let size=dword(section,16)?;let raw=dword(section,20)?;
        if let Some(relative)=rva.checked_sub(address){
            if relative.checked_add(length).is_some_and(|end|end<=size){
                let offset=raw.checked_add(relative).ok_or(BAD)?;span(bytes,offset,length)?;
                if result.replace(offset).is_some(){return Err(BAD)}
            }
        }
    }
    result.ok_or(BAD)
}
fn directory(resource:&[u8],offset:usize)->Result<Vec<(usize,usize)>,&'static str>{
    span(resource,offset,16)?;let count=word(resource,offset+12)?+word(resource,offset+14)?;
    if count>1024{return Err("PE_RESOURCE_SIZE_LIMIT")}
    let entries=span(resource,offset+16,count*8)?;
    entries.chunks_exact(8).map(|s|Ok((dword(s,0)?,dword(s,4)?))).collect()
}
struct Block<'a>{key:String,value:&'a [u8],children:&'a [u8],length:usize,kind:usize}
fn block(bytes:&[u8])->Result<Block<'_>,&'static str>{
    let length=word(bytes,0)?;if length<8{return Err(BAD)}let bytes=span(bytes,0,length)?;
    let units=word(bytes,2)?;let kind=word(bytes,4)?;if kind>1{return Err(BAD)}
    let mut offset=6;let mut key=Vec::new();
    loop{let value=word(bytes,offset)? as u16;offset+=2;if value==0{break}if key.len()>=128{return Err("PE_RESOURCE_SIZE_LIMIT")}key.push(value);}
    let key=String::from_utf16(&key).map_err(|_|BAD)?;let start=aligned(offset)?;
    let value_length=if kind==1{units.checked_mul(2).ok_or(BAD)?}else{units};
    let value=span(bytes,start,value_length)?;let child_offset=aligned(start.checked_add(value_length).ok_or(BAD)?)?;
    let children=if child_offset<=length{&bytes[child_offset..]}else if start+value_length==length{&[]}else{return Err(BAD)};
    Ok(Block{key,value,children,length,kind})
}
fn children<'a>(mut bytes:&'a [u8],budget:&mut usize)->Result<Vec<Block<'a>>,&'static str>{
    let mut result=Vec::new();
    while !bytes.is_empty(){
        if bytes.len()<=3&&bytes.iter().all(|v|*v==0){break}
        if *budget==0{return Err("PE_RESOURCE_SIZE_LIMIT")}*budget-=1;
        let b=block(bytes)?;let next=aligned(b.length)?;let length=b.length;
        result.push(b);bytes=if next<=bytes.len(){&bytes[next..]}else if length==bytes.len(){&[]}else{return Err(BAD)};
    }Ok(result)
}
fn retain(target:&mut Option<String>,b:&Block<'_>)->Result<(),&'static str>{
    if b.kind!=1||b.value.len()>514||!b.children.is_empty(){return Err(BAD)}
    let mut units:Vec<_>=b.value.chunks_exact(2).map(|s|u16::from_le_bytes([s[0],s[1]])).collect();
    if units.last()==Some(&0){units.pop();}
    let value=String::from_utf16(&units).map_err(|_|BAD)?;
    if value.chars().any(char::is_control){return Err(BAD)}
    if value.is_empty(){return Ok(())}
    if target.as_ref().is_some_and(|old|old!=&value){return Err("PE_VERSION_RESOURCE_CONFLICT")}
    *target=Some(value);Ok(())
}
fn version_resource(payload:&[u8],metadata:&mut PeMetadata,budget:&mut usize)->Result<(),&'static str>{
    let root=block(payload)?;if root.key!="VS_VERSION_INFO"||root.kind!=0{return Err(BAD)}
    if payload.len().saturating_sub(root.length)>3{return Err(BAD)}
    for info in children(root.children,budget)?{
        if info.key!="StringFileInfo"{continue}
        for table in children(info.children,budget)?{
            if table.key.len()!=8||!table.key.bytes().all(|c|c.is_ascii_hexdigit()){return Err(BAD)}
            for value in children(table.children,budget)?{
                let target=match value.key.as_str(){"ProductVersion"=>&mut metadata.product_version,"FileVersion"=>&mut metadata.file_version,"ProductName"=>&mut metadata.product_name,"CompanyName"=>&mut metadata.company_name,_=>continue};
                retain(target,&value)?;
            }
        }
    }Ok(())
}
/// Reads resource declarations only; these do not prove publisher or executable identity.
pub fn parse_pe_metadata(bytes:&[u8])->Result<PeMetadata,&'static str>{
    if bytes.len()>MAX_PE_BYTES{return Err("PE_RESOURCE_SIZE_LIMIT")}
    if span(bytes,0,2)?!=b"MZ"{return Err(BAD)}let pe=dword(bytes,60)?;
    if span(bytes,pe,4)?!=b"PE\0\0"{return Err(BAD)}
    let section_count=word(bytes,pe+6)?;if section_count==0||section_count>96{return Err(BAD)}
    let optional_length=word(bytes,pe+20)?;let optional=span(bytes,pe+24,optional_length)?;
    let dirs=match word(optional,0)?{0x10b=>96,0x20b=>112,_=>return Err(BAD)};
    if dword(optional,dirs-4)?<3{return Err("PE_VERSION_RESOURCE_NOT_FOUND")}
    let resource_rva=dword(optional,dirs+16)?;let resource_length=dword(optional,dirs+20)?;
    if resource_rva==0||resource_length==0{return Err("PE_VERSION_RESOURCE_NOT_FOUND")}
    if resource_length>2*1024*1024{return Err("PE_RESOURCE_SIZE_LIMIT")}
    let sections=span(bytes,pe+24+optional_length,section_count*40)?;
    let offset=map_rva(bytes,sections,resource_rva,resource_length)?;let resource=span(bytes,offset,resource_length)?;
    let mut metadata=PeMetadata::default();let mut leaves=0;let mut budget=2048;
    for (kind,next) in directory(resource,0)?{
        if kind!=16{continue}if next&0x80000000==0{return Err(BAD)}
        for (_,next) in directory(resource,next&0x7fffffff)?{
            if next&0x80000000==0{return Err(BAD)}
            for (_,leaf) in directory(resource,next&0x7fffffff)?{
                if leaf&0x80000000!=0{return Err(BAD)}leaves+=1;if leaves>8{return Err("PE_RESOURCE_SIZE_LIMIT")}
                span(resource,leaf,16)?;let rva=dword(resource,leaf)?;let size=dword(resource,leaf+4)?;
                let relative=rva.checked_sub(resource_rva).ok_or(BAD)?;
                version_resource(span(resource,relative,size)?,&mut metadata,&mut budget)?;
            }
        }
    }
    if leaves==0{return Err("PE_VERSION_RESOURCE_NOT_FOUND")}Ok(metadata)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn align(v:&mut Vec<u8>){while v.len()%4!=0{v.push(0)}}
    fn block(key:&str,value:&[u8],value_length:u16,kind:u16,children:&[Vec<u8>])->Vec<u8>{
        let mut v=vec![0,0];v.extend(value_length.to_le_bytes());v.extend(kind.to_le_bytes());
        for c in key.encode_utf16().chain(Some(0)){v.extend(c.to_le_bytes())}align(&mut v);v.extend(value);align(&mut v);
        for child in children{v.extend(child);align(&mut v)}let length=v.len() as u16;v[..2].copy_from_slice(&length.to_le_bytes());v
    }
    fn strings(values:&[(&str,&str)])->Vec<u8>{
        let children:Vec<_>=values.iter().map(|(key,value)|{let bytes:Vec<_>=value.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect();block(key,&bytes,(bytes.len()/2) as u16,1,&[])}).collect();
        let table=block("040904b0",&[],0,1,&children);let info=block("StringFileInfo",&[],0,1,&[table]);block("VS_VERSION_INFO",&[],0,0,&[info])
    }
    fn fixture(is64:bool,values:&[(&str,&str)])->Vec<u8>{
        let payload=strings(values);let mut resource=vec![0u8;88];
        for (offset,value) in [(14,1u16),(38,1),(62,1)]{resource[offset..offset+2].copy_from_slice(&value.to_le_bytes())}
        for (offset,value) in [(16,16u32),(20,0x80000018),(40,1),(44,0x80000030),(64,1033),(68,72),(72,0x1058),(76,payload.len() as u32)]{resource[offset..offset+4].copy_from_slice(&value.to_le_bytes())}
        resource.extend(payload);let optional_size=if is64{240}else{224};let mut pe=vec![0u8;512+resource.len()];
        pe[..2].copy_from_slice(b"MZ");pe[60..64].copy_from_slice(&128u32.to_le_bytes());pe[128..132].copy_from_slice(b"PE\0\0");
        pe[134..136].copy_from_slice(&1u16.to_le_bytes());pe[148..150].copy_from_slice(&(optional_size as u16).to_le_bytes());
        pe[152..154].copy_from_slice(&(if is64{0x20bu16}else{0x10b}).to_le_bytes());let directories=152+if is64{112}else{96};
        pe[directories-4..directories].copy_from_slice(&16u32.to_le_bytes());pe[directories+16..directories+20].copy_from_slice(&0x1000u32.to_le_bytes());pe[directories+20..directories+24].copy_from_slice(&(resource.len() as u32).to_le_bytes());
        let section=152+optional_size;for (offset,value) in [(8,resource.len() as u32),(12,0x1000),(16,resource.len() as u32),(20,512)]{pe[section+offset..section+offset+4].copy_from_slice(&value.to_le_bytes())}
        pe[512..].copy_from_slice(&resource);pe
    }
    #[test]fn pe32_and_pe64_observe_exact_resource_strings_without_execution(){
        for is64 in [false,true]{let p=parse_pe_metadata(&fixture(is64,&[("ProductVersion","3.13.2"),("FileVersion","3.13.2150.1013"),("ProductName","Python"),("CompanyName","Python Software Foundation")])).unwrap();
            assert_eq!(p.product_version.as_deref(),Some("3.13.2"));assert_eq!(p.file_version.as_deref(),Some("3.13.2150.1013"));assert_eq!(p.product_name.as_deref(),Some("Python"));assert_eq!(p.company_name.as_deref(),Some("Python Software Foundation"));}
    }
    #[test]fn duplicate_conflicting_version_strings_cannot_establish_one_version(){
        assert_eq!(parse_pe_metadata(&fixture(false,&[("ProductVersion","3.13.2"),("ProductVersion","3.12.0")])).err(),Some("PE_VERSION_RESOURCE_CONFLICT"));
    }
    #[test]fn malformed_offsets_truncation_and_absence_have_stable_errors(){
        let valid=fixture(false,&[("ProductVersion","3.13.2")]);for end in [0,2,60,128,152,200,512,valid.len()-1]{assert!(parse_pe_metadata(&valid[..end]).is_err());}
        let mut poisoned=valid.clone();poisoned[60..64].copy_from_slice(&u32::MAX.to_le_bytes());assert!(parse_pe_metadata(&poisoned).is_err());
        poisoned=valid.clone();poisoned[532..536].copy_from_slice(&u32::MAX.to_le_bytes());assert!(parse_pe_metadata(&poisoned).is_err());
        poisoned=valid;poisoned[264..268].fill(0);assert!(parse_pe_metadata(&poisoned).is_err());
    }
    #[test]fn irrelevant_private_resource_fields_are_not_retained_and_mutations_never_panic(){
        let valid=fixture(true,&[("ProductVersion","3.13.2"),("PrivatePath","C:\\private\\secret"),("Comments","user:secret@host")]);
        let p=parse_pe_metadata(&valid).unwrap();let json=serde_json::to_string(&p).unwrap();assert!(!json.contains("private")&&!json.contains("secret"));
        for i in (0..valid.len()).step_by(7){let mut input=valid.clone();input[i]^=0xff;assert!(std::panic::catch_unwind(||parse_pe_metadata(&input)).is_ok());}
    }
}
