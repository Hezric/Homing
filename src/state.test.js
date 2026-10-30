import test from 'node:test';
import assert from 'node:assert/strict';
import {mockState,mockUpdate,parsePauseMinutes,pauseLabel,validProfile,localeFor,connectionLabel} from './state.js';
test('pause durations and locale follow the Earmuff contract',()=>{
 for(const value of ['0','721','1.5','-1','junk',''])assert.equal(parsePauseMinutes(value),null);
 for(const [value,expected] of [['1',1],[' 45 ',45],['720',720]])assert.equal(parsePauseMinutes(value),expected);
 assert.equal(pauseLabel(60,'en'),'Pause 1 hr');assert.equal(pauseLabel(45,'zh'),'暂停 45 分钟');
 assert.equal(localeFor('zh-TW'),'zh');assert.equal(localeFor('en-US'),'en');
});
test('mock pause and duration edits preserve the current deadline',()=>{
 const state=mockUpdate(mockState(),{type:'togglePause'});
 assert.equal(state.pauseRemaining,3600);
 mockUpdate(state,{type:'setPauseMinutes',minutes:45});assert.equal(state.pauseRemaining,3600);
 mockUpdate(state,{type:'togglePause'});assert.equal(state.pauseRemaining,0);
 mockUpdate(state,{type:'togglePause'});assert.equal(state.pauseRemaining,2700);
});
test('profile validation preserves names and rejects line breaks',()=>{
 for(const value of ['', ' ', 'Home\n','Home\0','a'.repeat(256)])assert.equal(validProfile(value),false);
 assert.equal(validProfile('家 & " = [x]'),true);
});
test('current Wi-Fi connection is distinct from target reconnect state',()=>{
 assert.equal(connectionLabel({...mockState('disconnected'),current:'Away Wi-Fi'},'en'),'Connected');
 assert.equal(connectionLabel({...mockState('disconnected'),current:null},'zh'),'无法连接 Home Wi-Fi · 自动重试中');
 assert.equal(connectionLabel(mockState('reconnecting'),'en'),'Reconnecting…');
});
test('unreadable Wi-Fi state is distinct from disconnected',()=>{
 assert.equal(connectionLabel({...mockState('disconnected'),connection:'unreadable'},'en'),"Can't read Wi-Fi state");
 assert.equal(connectionLabel({...mockState('disconnected'),connection:'unreadable'},'zh'),'无法读取 Wi-Fi 状态');
});

test('unreachable target explains automatic retry and missing configuration',()=>{
 assert.equal(connectionLabel(mockState('disconnected'),'en'),"Can't reach Home Wi-Fi · retrying automatically");
 assert.equal(connectionLabel(mockState('disconnected'),'zh'),'无法连接 Home Wi-Fi · 自动重试中');
 assert.equal(connectionLabel({...mockState('disconnected'),profile:''},'en'),'Choose a network to keep');
 assert.equal(connectionLabel({...mockState('disconnected'),profile:''},'zh'),'选择要保持连接的网络');
});
