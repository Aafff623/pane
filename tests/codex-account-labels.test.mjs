import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=readFileSync(new URL('../src/main.ts',import.meta.url),'utf8');
const ast=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const names=['managedAccountForCard','labelForAccount'];
const code=ts.transpile(ast.statements.filter(n=>ts.isFunctionDeclaration(n)&&names.includes(n.name?.text)).map(n=>n.getText(ast)).join('\n'),{target:ts.ScriptTarget.ES2020});
function label(id,rows,note='') {
 const ctx=vm.createContext({accountNote:()=>note,providerFamily:id=>id.split('@')[0],providerDefinition:()=>({supportsApiKey:false}),isWebSessionFamily:()=>false,providerDisplayName:()=>'ChatGPT',t:()=> 'Account'});
 vm.runInContext(code,ctx);return ctx.labelForAccount(id,rows);
}
test('bare Codex uses complete desktop email and source from account metadata',()=>{
 assert.equal(label('codex',[{id:'codex',label:'desktop@example.com · 桌面',email:'desktop@example.com'},{id:'codex@web',label:'web@example.com'}]),'desktop@example.com');
});
test('bare Codex never borrows first web identity when local identity is missing',()=>{
 assert.equal(label('codex',[{id:'codex@web',label:'web@example.com'}]),'ChatGPT');
});
test('Codex identity takes precedence over stale provider notes after switching desktop login',()=>{
 assert.equal(label('codex',[{id:'codex',label:'desktop@example.com · 桌面',email:'desktop@example.com'}],'ChatGPT'),'desktop@example.com');
 assert.equal(label('codex',[{id:'codex',email:'second@example.com'}],'Work'),'second@example.com');
});
