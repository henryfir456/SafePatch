import {render,screen,fireEvent,waitFor} from '@testing-library/react';
import {beforeEach,expect,test,vi} from 'vitest';
import {NpmInventory} from './NpmInventory';
import {npmApi} from './npmApi';
import {api} from './api';
import type {DependencyGraph} from './npmApi';
import type {Scan} from './api';
vi.mock('./npmApi',()=>({npmApi:{latest:vi.fn(),graph:vi.fn(),start:vi.fn()}}));
vi.mock('./api',()=>({api:{status:vi.fn(),cancel:vi.fn()}}));
const scan:Scan={id:'npm-1',status:'PARTIAL',phase:'finished',progress:100,started_at:'2026-10-10T00:00:00Z',finished_at:null,coverage:[],error_code:null};
const graph:DependencyGraph={environment_id:'e'.repeat(64),root_path_hash:'f'.repeat(64),lockfile_hash:'a'.repeat(64),lockfile_kind:'package-lock',lockfile_version:3,observed_at:scan.started_at,packages:[{id:'p1',name:'example',version:'1.2.3',direct_status:'DIRECT',source:'NPM_REGISTRY_METADATA',slot_hash:'b'.repeat(64)},{id:'p2',name:'leaf',version:null,direct_status:'UNKNOWN',source:'UNKNOWN',slot_hash:'c'.repeat(64)}],edges:[{from_id:'p1',to_id:'p2',kinds:['OPTIONAL']}],codes:['LOCKFILE_DECLARATIONS_NOT_INSTALLED_PROOF']};
beforeEach(()=>{vi.resetAllMocks();vi.mocked(npmApi.latest).mockResolvedValue(null);vi.mocked(npmApi.graph).mockResolvedValue(null);});
test('project scan is explicit and errors do not reveal private paths',async()=>{
  render(<NpmInventory disabled={false} onActive={vi.fn()}/>);await waitFor(()=>expect(npmApi.latest).toHaveBeenCalled());expect(npmApi.start).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('npm project directory'),{target:{value:'C:\\chosen'}});
  vi.mocked(npmApi.start).mockRejectedValue(new Error('C:\\private\\secret-token'));fireEvent.click(screen.getByRole('button',{name:'開始 npm 唯讀掃描'}));
  expect(await screen.findByRole('alert')).toHaveTextContent('NPM_START_FAILED');expect(screen.queryByText(/secret-token/)).toBeNull();expect(npmApi.start).toHaveBeenCalledWith('C:\\chosen');
});
test('restart restores exact declarations and graph uncertainty, with relation filtering',async()=>{
  vi.mocked(npmApi.latest).mockResolvedValue(scan);vi.mocked(npmApi.graph).mockResolvedValue(graph);render(<NpmInventory disabled={false} onActive={vi.fn()}/>);
  expect(await screen.findByText('1.2.3')).toBeInTheDocument();expect(screen.getByText('LOCKFILE_DECLARATIONS_NOT_INSTALLED_PROOF')).toBeInTheDocument();
  fireEvent.click(screen.getAllByRole('button',{name:'查看依賴'})[0]);expect(screen.getByLabelText('npm outgoing dependencies')).toHaveTextContent('leaf · UNKNOWN · OPTIONAL');
  fireEvent.click(screen.getByRole('radio',{name:'UNKNOWN'}));expect(screen.queryByText('1.2.3')).toBeNull();expect(screen.getByRole('table')).toHaveTextContent('leaf');
});
test('real command progress/cancel ids and terminal graph refresh are connected',async()=>{
  vi.mocked(npmApi.start).mockResolvedValue({...scan,status:'RUNNING',progress:10});vi.mocked(api.status).mockResolvedValue(scan);vi.mocked(npmApi.graph).mockResolvedValue(graph);
  render(<NpmInventory disabled={false} onActive={vi.fn()}/>);fireEvent.change(screen.getByLabelText('npm project directory'),{target:{value:'C:\\chosen'}});
  await waitFor(()=>expect(screen.getByRole('button',{name:'開始 npm 唯讀掃描'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'開始 npm 唯讀掃描'}));
  fireEvent.click(await screen.findByRole('button',{name:'取消 npm 掃描'}));await waitFor(()=>expect(api.cancel).toHaveBeenCalledWith('npm-1'));
  expect(await screen.findByText('1.2.3')).toBeInTheDocument();expect(npmApi.graph).toHaveBeenCalledWith('npm-1');expect(api.status).toHaveBeenCalledWith('npm-1');
});
test('large declaration list pages without dropping data and poisoned text stays inert',async()=>{
  const packages=Array.from({length:250},(_,i)=>({...graph.packages[0],id:'p'+i,name:i===201?'<img src=x onerror=alert(1)>':'package-'+i}));
  vi.mocked(npmApi.latest).mockResolvedValue(scan);vi.mocked(npmApi.graph).mockResolvedValue({...graph,packages,edges:[]});
  const {container}=render(<NpmInventory disabled={false} onActive={vi.fn()}/>);expect(await screen.findByText('package-0')).toBeInTheDocument();expect(screen.getAllByRole('row')).toHaveLength(201);
  fireEvent.click(screen.getByRole('button',{name:'下一頁'}));expect(screen.getByText('package-249')).toBeInTheDocument();expect(screen.queryByText('package-0')).toBeNull();expect(screen.getAllByRole('row')).toHaveLength(51);expect(container.querySelector('img')).toBeNull();
  fireEvent.click(screen.getByRole('button',{name:'上一頁'}));expect(screen.getByText('package-0')).toBeInTheDocument();
});
