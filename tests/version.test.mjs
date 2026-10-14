import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import assert from 'node:assert/strict';
const json=p=>JSON.parse(readFileSync(new URL('../'+p,import.meta.url),'utf8'));
function aligned(packageVersion,cargoVersion,tauriVersion,lockVersion){assert.match(packageVersion,/^\d+\.\d+\.\d+(?:-preview)?$/);for(const v of [cargoVersion,tauriVersion,lockVersion])assert.equal(v,packageVersion);}
test('package.json is authoritative and all installer/binary manifests agree',()=>{
 const pkg=json('package.json'),conf=json('src-tauri/tauri.conf.json'),lock=json('package-lock.json');
 const cargo=readFileSync(new URL('../src-tauri/Cargo.toml',import.meta.url),'utf8').match(/^version = "([^"]+)"/m)[1];
 aligned(pkg.version,cargo,conf.version,lock.version);assert.equal(lock.packages[''].version,pkg.version);
 assert.equal(conf.bundle.windows.wix.version,pkg.version.split('-')[0]);
 const rustLock=readFileSync(new URL('../src-tauri/Cargo.lock',import.meta.url),'utf8').replaceAll('\r\n','\n');assert.ok(rustLock.includes('name = "safepatch"\nversion = "'+pkg.version+'"'));
});
test('old package versions and incorrect channel fail alignment',()=>{for(const v of ['0.2.0','0.3.0','garbage'])assert.throws(()=>aligned('0.3.0-preview',v,'0.3.0-preview','0.3.0-preview'));});
test('frontend reads embedded backend metadata, never runtime Git or hardcoded version',()=>{
 const ui=readFileSync(new URL('../src/BuildInfo.tsx',import.meta.url),'utf8');assert.ok(ui.includes("'build_info'"));assert.ok(!ui.includes('0.3.0'));assert.ok(!ui.includes('fetch('));
 const source=readFileSync(new URL('../src-tauri/src/build_info.rs',import.meta.url),'utf8');assert.ok(source.includes('env!("CARGO_PKG_VERSION")'));assert.ok(!source.includes('Command::new'));
});
