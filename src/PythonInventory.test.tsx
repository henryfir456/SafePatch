import {render,screen,fireEvent,waitFor} from '@testing-library/react';
import {beforeEach,expect,test,vi} from 'vitest';
import {PythonInventory} from './PythonInventory';
import {pythonApi} from './pythonApi';
import {api} from './api';
import type {PythonSnapshot} from './pythonApi';
import type {Scan} from './api';
vi.mock('./pythonApi',()=>({pythonApi:{latest:vi.fn(),snapshot:vi.fn(),start:vi.fn()}}));
vi.mock('./api',()=>({api:{status:vi.fn(),cancel:vi.fn()}}));
const scan:Scan={id:'python-1',status:'PARTIAL',phase:'finished',progress:100,started_at:'2026-10-10T00:00:00Z',finished_at:null,coverage:[],error_code:null};
const snapshot:PythonSnapshot={environment_id:'e'.repeat(64),root_path_hash:'f'.repeat(64),observed_at:scan.started_at,environment_kind:'VENV_METADATA',venv_metadata:{declared_version:'3.13.2',include_system_packages:true,codes:['BASE_AND_EXTRA_SITE_PACKAGES_NOT_SCANNED']},interpreters:[{id:'i',path_hash:'a'.repeat(64),version:null,file_version:null,product_name:'Python',publisher:null,source:'PE_VERSION_RESOURCE_METADATA',codes:['PYTHON_INTERPRETER_VERSION_UNKNOWN']}],packages:[{id:'p',name:'demo',display_name:'Demo',version:'1.0',direct_status:'UNKNOWN',source:'DIST_INFO_METADATA',installer:'PIP_INSTALLER_METADATA',slot_hash:'b'.repeat(64),codes:['INSTALLED_METADATA_NOT_CONTENT_PROOF']}],codes:['BASE_AND_EXTRA_SITE_PACKAGES_NOT_SCANNED']};
beforeEach(()=>{vi.resetAllMocks();vi.mocked(pythonApi.latest).mockResolvedValue(null);vi.mocked(pythonApi.snapshot).mockResolvedValue(null);});
test('environment selection is opt-in and raw backend errors stay private',async()=>{
  render(<PythonInventory disabled={false} onActive={vi.fn()}/>);await waitFor(()=>expect(pythonApi.latest).toHaveBeenCalled());expect(pythonApi.start).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('Python environment directory'),{target:{value:'C:\\chosen'}});vi.mocked(pythonApi.start).mockRejectedValue(new Error('C:\\private\\secret-token'));fireEvent.click(screen.getByRole('button',{name:'Scan Python metadata'}));
  expect(await screen.findByRole('alert')).toHaveTextContent('PYTHON_START_FAILED');expect(screen.queryByText(/secret-token/)).toBeNull();expect(pythonApi.start).toHaveBeenCalledWith('C:\\chosen');
});
test('venv hint is separate from unknown observed interpreter and dependency relation',async()=>{
  vi.mocked(pythonApi.latest).mockResolvedValue(scan);vi.mocked(pythonApi.snapshot).mockResolvedValue(snapshot);render(<PythonInventory disabled={false} onActive={vi.fn()}/>);
  expect(await screen.findByText('1.0')).toBeInTheDocument();expect(screen.getByLabelText('Python interpreter observations')).toHaveTextContent('UNKNOWN');expect(screen.getByLabelText('Python venv metadata hint')).toHaveTextContent('3.13.2');expect(screen.getByRole('table',{name:'Python installed distribution metadata'})).toHaveTextContent('UNKNOWN');expect(screen.getAllByText('BASE_AND_EXTRA_SITE_PACKAGES_NOT_SCANNED').length).toBeGreaterThan(0);
});
test('cancel and terminal refresh use the active real command id',async()=>{
  vi.mocked(pythonApi.start).mockResolvedValue({...scan,status:'RUNNING',progress:10});vi.mocked(api.status).mockResolvedValue(scan);vi.mocked(pythonApi.snapshot).mockResolvedValue(snapshot);
  render(<PythonInventory disabled={false} onActive={vi.fn()}/>);fireEvent.change(screen.getByLabelText('Python environment directory'),{target:{value:'C:\\chosen'}});await waitFor(()=>expect(screen.getByRole('button',{name:'Scan Python metadata'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'Scan Python metadata'}));fireEvent.click(await screen.findByRole('button',{name:'Cancel Python scan'}));await waitFor(()=>expect(api.cancel).toHaveBeenCalledWith('python-1'));expect(await screen.findByText('1.0')).toBeInTheDocument();expect(pythonApi.snapshot).toHaveBeenCalledWith('python-1');
});
test('large installed metadata list is paged and poisoned text remains inert',async()=>{
  vi.mocked(pythonApi.latest).mockResolvedValue(scan);vi.mocked(pythonApi.snapshot).mockResolvedValue({...snapshot,packages:Array.from({length:250},(_,i)=>({...snapshot.packages[0],id:'p'+i,name:'package-'+i,display_name:i===201?'<img src=x onerror=alert(1)>':'package-'+i}))});
  const {container}=render(<PythonInventory disabled={false} onActive={vi.fn()}/>);expect(await screen.findByText('package-0')).toBeInTheDocument();expect(screen.getAllByRole('row')).toHaveLength(201);fireEvent.click(screen.getByRole('button',{name:'Next Python page'}));expect(screen.getByText('package-249')).toBeInTheDocument();expect(screen.getAllByRole('row')).toHaveLength(51);expect(container.querySelector('img')).toBeNull();
});
