// Deterministic application-owned ICO; no external assets or generators.
import { mkdirSync, writeFileSync } from 'node:fs';
const size=32, pixels=Buffer.alloc(size*size*4), mask=Buffer.alloc(size*4), bmp=Buffer.alloc(40);
for(let y=0;y<size;y++)for(let x=0;x<size;x++){
  const shield=y>=5&&y<=25&&Math.abs(x-15.5)<(y<17?10:10-(y-17)*1.1);
  const check=shield&&((x>=10&&x<=15&&Math.abs(y-(x+4))<1.5)||(x>=15&&x<=22&&Math.abs(y-(34-x))<1.5));
  const color=check?[16,32,40]:shield?[116,209,195]:[16,25,35];const i=((size-1-y)*size+x)*4;
  pixels[i]=color[2];pixels[i+1]=color[1];pixels[i+2]=color[0];pixels[i+3]=255;
}
bmp.writeUInt32LE(40,0);bmp.writeInt32LE(size,4);bmp.writeInt32LE(size*2,8);bmp.writeUInt16LE(1,12);bmp.writeUInt16LE(32,14);bmp.writeUInt32LE(pixels.length,20);
const data=Buffer.concat([bmp,pixels,mask]), header=Buffer.alloc(22);header.writeUInt16LE(1,2);header.writeUInt16LE(1,4);header[6]=size;header[7]=size;header.writeUInt16LE(1,10);header.writeUInt16LE(32,12);header.writeUInt32LE(data.length,14);header.writeUInt32LE(22,18);
mkdirSync(new URL('../src-tauri/icons/',import.meta.url),{recursive:true});writeFileSync(new URL('../src-tauri/icons/icon.ico',import.meta.url),Buffer.concat([header,data]));
