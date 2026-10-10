//! PE resource data is parsed from bounded bytes; no loader or binary runs.
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
