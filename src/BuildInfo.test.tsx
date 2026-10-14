import {render,screen,fireEvent} from '@testing-library/react';
import {test,expect,vi,beforeEach} from 'vitest';
import {invoke} from '@tauri-apps/api/core';
import {BuildInfo} from './BuildInfo';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn()}));
beforeEach(()=>vi.resetAllMocks());
test('header/footer/about display the binary version and full embedded SHA',async()=>{
 vi.mocked(invoke).mockResolvedValue({version:'9.2.1-preview',build_sha:'12345678'+'a'.repeat(32),dirty:false,channel:'Preview',release_url:'https://github.com/henryfir456/SafePatch/releases/tag/v9.2.1-preview'});
 render(<><BuildInfo location="header"/><BuildInfo location="footer"/></>);
 expect(await screen.findByTestId('app-version')).toHaveTextContent('v9.2.1-preview');
 fireEvent.click(screen.getByText('關於 SafePatch'));expect(screen.getByText(/完整版本/)).toHaveTextContent('12345678'+'a'.repeat(32));
 expect(screen.getByText(/^版本：v9/)).toHaveTextContent('Build：12345678');
});
test('missing backend metadata does not claim a release version',async()=>{vi.mocked(invoke).mockRejectedValue('BACKEND_UNAVAILABLE');render(<BuildInfo location="header"/>);expect(screen.getByTestId('app-version')).toHaveTextContent('版本資訊未取得');});
