// Read-only aggregate checks of the owned CI application's actual database (#22).
import {DatabaseSync} from 'node:sqlite';
const db=new DatabaseSync(process.argv[2],{readOnly:true});
const schema=db.prepare('PRAGMA user_version').get().user_version;
const latest=scope=>db.prepare("SELECT id,status FROM scan_runs WHERE scope=? ORDER BY started_at DESC,id DESC LIMIT 1").get(scope)??null;
const windows=latest('windows'),npm=latest('npm');
const assets=windows?db.prepare('SELECT count(*) n FROM scan_assets WHERE scan_run_id=?').get(windows.id).n:0;
const graph=npm?db.prepare('SELECT snapshot_json FROM scan_project_graphs WHERE scan_run_id=?').get(npm.id):null;
const parsed=graph?JSON.parse(graph.snapshot_json):null;
console.log(JSON.stringify({schema,windows,npm,assets,packages:parsed?.packages.length??0,edges:parsed?.edges.length??0,fk_errors:db.prepare('PRAGMA foreign_key_check').all().length}));
db.close();
