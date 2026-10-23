import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=readFileSync(new URL('../src/main.ts',import.meta.url),'utf8');
test('startup config and native locale requests run concurrently; optional locale failure does not block config',async()=>{
 const ast=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
 const fn=ast.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='loadStartupPreferences');
 assert.ok(fn,'startup must not serialize independent native reads');
 const calls=[];let finish;
 const ctx=vm.createContext({invoke:cmd=>{calls.push(cmd);return cmd==='get_config'?new Promise(r=>finish=r):Promise.reject(Error('optional locale unavailable'))},setSystemLocale(){}});
 vm.runInContext(ts.transpile(fn.getText(),{target:ts.ScriptTarget.ES2020}),ctx);
 const result=ctx.loadStartupPreferences();
 assert.deepEqual(calls,['get_config','system_ui_locale']);
 finish({locale:'zh'});assert.equal((await result).locale,'zh');
});
