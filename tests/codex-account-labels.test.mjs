import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=readFileSync(new URL('../src/main.ts',import.meta.url),'utf8');
const ast=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
function functions(...names) {
 const code=ts.transpile(ast.statements.filter(n=>ts.isFunctionDeclaration(n)&&names.includes(n.name?.text)).map(n=>n.getText(ast)).join('\n'),{target:ts.ScriptTarget.ES2020});
 return code;
}
function label(id,rows,note='') {
 const ctx=vm.createContext({accountNote:()=>note,providerFamily:id=>id.split('@')[0],providerDefinition:()=>({supportsApiKey:false}),isWebSessionFamily:()=>false,providerDisplayName:()=>'ChatGPT',t:()=> 'Account'});
 vm.runInContext(functions('managedAccountForCard','labelForAccount'),ctx);return ctx.labelForAccount(id,rows);
}
test('bare Codex uses the Google/ChatGPT email from account metadata',()=>{
 assert.equal(label('codex',[{id:'codex',label:'desktop@example.com · 桌面',email:'desktop@example.com'},{id:'codex@web',label:'web@example.com'}]),'desktop@example.com');
});
test('an account without identity yields no fabricated account name',()=>{
 assert.equal(label('codex',[{id:'codex@web',label:'web@example.com'}]),'');
});
test('an explicit account note wins over the email',()=>{
 assert.equal(label('codex',[{id:'codex',email:'desktop@example.com'}],'Work'),'Work');
});

function noteContext(list, notes) {
 const accountsCache=new Map([['codex',list]]);
 const config={layout:{providers:Object.fromEntries(Object.entries(notes).map(([key,value])=>[key,{note:value}]))}};
 const ctx=vm.createContext({
  accountsCache,config,NOTE_MAX_CODE_POINTS:48,
  providerFamily:id=>id.split('@')[0],
  providerDefinition:()=>({supportsApiKey:false}),
  isWebSessionFamily:()=>false,
  t:()=> 'Account',
 });
 vm.runInContext(functions('managedAccountForCard','normalizeNote','cardNote','accountNoteKeys','accountNoteKey','accountNote'),ctx);
 return ctx;
}

test('the provider card note never becomes the default account note',()=>{
 const ctx=noteContext([{id:'codex',email:'desktop@example.com'},{id:'codex@3581686f',email:'web@example.com'}],{codex:'ChatGPT'});
 assert.equal(ctx.accountNote('codex'),'','"ChatGPT" is the provider title, not this account');
 assert.equal(ctx.accountNote('codex@3581686f'),'');
 assert.equal(ctx.accountNoteKeys('codex').join(','),'codex@__default__');
 assert.equal(ctx.accountNoteKey('codex'),'codex@__default__','renaming the account must not rewrite the provider key');
});

test('an explicit default-account note stays readable and wins over the email',()=>{
 const ctx=noteContext([{id:'codex',email:'desktop@example.com'}],{'codex@__default__':'Work'});
 assert.equal(ctx.accountNote('codex'),'Work');
});
