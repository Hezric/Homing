import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { localeFor, mockState, mockUpdate, text, parsePauseMinutes, pauseLabel, validProfile, connectionLabel } from './state.js';
import './style.css';
const glyphs=import.meta.glob('../assets/fluent-v1/*.svg',{query:'?raw',import:'default',eager:true});
const icon=(name,size=24)=>glyphs[`../assets/fluent-v1/${name}_${size}_regular.svg`].replace('<svg ','<svg aria-hidden="true" focusable="false" ');
const native=isTauri(), params=new URLSearchParams(location.search);
const systemLanguage=localeFor(native ? navigator.language : params.get('lang') || navigator.language);
let language=systemLanguage;
let state=native ? {...mockState('disconnected'),current:null,profile:'',lastReconnect:null} : mockState(['connected','reconnecting','disconnected','paused','unreadable'].includes(params.get('state')) ? params.get('state') : 'connected');
let settingsOpen=!native && params.get('view')==='settings', localError='', pauseInvalid=false, profileInvalid=false, lastHeight=0;
let fitRequest=Promise.resolve(), flyoutLayout;
const reducedMotion=matchMedia('(prefers-reduced-motion: reduce)');
const motion={duration:200,easing:'cubic-bezier(0.16, 1, 0.3, 1)'};
let motionReady=false;
document.body.classList.toggle('native',native);
if(!native && ['light','dark'].includes(params.get('theme'))) document.documentElement.dataset.theme=params.get('theme');
const root=document.querySelector('#app');
root.innerHTML=`<section class="flyout" aria-label="Homing">
<header class="header"><span class="brand-icon">${icon('wifi_1',20)}</span><span class="brand">Homing</span></header>
<div class="current"><span class="badge" id="current-icon"></span><div class="current-copy"><div id="current-name"></div><p id="current-status" role="status"></p></div></div>
<div id="views"><section id="network-view"><div class="device-heading"><h1 id="reconnect-label"></h1><span id="mode" class="mode"></span></div><section class="device-list" aria-labelledby="reconnect-label"><div class="device-row"><span class="device-icon">${icon('wifi_1',20)}</span><span id="target-label" class="row-label"></span><span class="device-name row-value" id="target"></span></div><div class="device-row"><span class="device-icon">${icon('arrow_sync',16)}</span><span id="last-label" class="row-label"></span><time class="device-name row-value" id="last"></time></div></section><div id="setup-wrap" class="message" aria-hidden="true"><div><p id="setup" class="setup"></p></div></div></section>
<section id="settings" hidden><div class="settings"><h2><label id="profile-label" for="profile"></label></h2><div class="profile-row"><input id="profile" class="profile" type="text" maxlength="255" autocomplete="off" spellcheck="false" aria-describedby="profile-hint profile-error"><button class="button" id="save-profile" type="button"></button></div><p id="profile-hint" class="profile-hint"></p><div id="profile-error-wrap" class="message" aria-hidden="true"><div><p id="profile-error" class="profile-error" role="alert"></p></div></div>
<h2 id="pause-for-label"></h2><div class="pause-row"><div class="segments presets" role="group" aria-labelledby="pause-for-label"><button data-minutes="15">15</button><button data-minutes="30">30</button><button data-minutes="60">60</button><button data-minutes="120">120</button></div><input id="pause-minutes" class="pause-minutes" type="text" inputmode="numeric" maxlength="4" aria-labelledby="pause-for-label" aria-describedby="pause-error" autocomplete="off" spellcheck="false"><span id="minutes-unit" class="minutes-unit"></span></div><div id="pause-error-wrap" class="message" aria-hidden="true"><div><p id="pause-error" class="pause-error" role="alert"></p></div></div>
<h2 id="language-label"></h2><div class="segments" role="group" aria-labelledby="language-label"><button data-language="system"></button><button data-language="zh">简体中文</button><button data-language="en">English</button></div><div class="about"><span>Homing 0.1.0</span><a id="github" href="https://github.com/Hezric/Homing" target="_blank" rel="noreferrer">GitHub ${icon('open')}</a></div>
</div></section></div><div id="error-wrap" class="message" aria-hidden="true"><div><p id="error" role="alert"></p></div></div><div class="footer-area"><div class="divider"></div><footer><button id="pause" class="button" type="button"></button><button id="gear" class="button icon-button" aria-expanded="false" aria-controls="settings">${icon('settings')}</button></footer></div></section>`;
const contentValues = new WeakMap();
function setContent(id, value, html = false) {
  const element = typeof id === 'string' ? document.getElementById(id) : id;
  value = String(value);
  if (contentValues.get(element) === value) return;
  const old = element.querySelector(':scope > .copy-current');
  const oldPosition = old ? [old.offsetLeft, old.offsetTop] : [0, 0];
  const oldWidth = element.getBoundingClientRect().width;
  const oldHeight = element.getBoundingClientRect().height;
  element.querySelectorAll(':scope > .copy-outgoing').forEach(node => node.remove());
  const content = document.createElement('span'); content.className = 'copy-current';
  if (html) content.innerHTML = value; else content.textContent = value;
  element.replaceChildren(content);
  contentValues.set(element, value);
  element.classList.add('motion-copy');
  if (old && motionReady && !reducedMotion.matches) {
    if ((element.id === 'pause' || element.id === 'mode')) element.animate([{ width: `${oldWidth}px` }, { width: `${element.getBoundingClientRect().width}px` }], motion);
    const height = element.getBoundingClientRect().height;
    if ((element.closest('.message') || element.id === 'profile-hint') && Math.abs(height - oldHeight) > .5) element.animate([{ height: `${oldHeight}px` }, { height: `${height}px` }], motion);
    old.className = 'copy-outgoing'; old.style.left = `${oldPosition[0]}px`; old.style.top = `${oldPosition[1]}px`; old.setAttribute('aria-hidden', 'true'); element.append(old);
    old.animate([{ opacity: 1 }, { opacity: 0 }], { ...motion, fill: 'forwards' }).finished.finally(() => old.remove()).catch(() => {});
    content.animate([{ opacity: 0, transform: 'translateY(3px)' }, { opacity: 1, transform: 'none' }], motion);
  }
}
const setText = (id, value) => setContent(id, value);
function animateView(before, outgoing) {
  const views = document.querySelector('#views');
  views.getAnimations().forEach(animation => animation.cancel());
  views.querySelectorAll(':scope > .view-outgoing').forEach(node => node.remove());
  const height = views.getBoundingClientRect().height;
  if (!motionReady || reducedMotion.matches) return;
  if (Math.abs(height - before) > .5) views.animate([{ height: `${before}px` }, { height: `${height}px` }], motion);
  if (outgoing) {
    outgoing.removeAttribute('id'); outgoing.querySelectorAll('[id]').forEach(node => node.removeAttribute('id'));
    outgoing.classList.add('view-outgoing'); outgoing.inert = true; outgoing.setAttribute('aria-hidden', 'true');
    outgoing.style.height = `${before}px`; views.append(outgoing);
    outgoing.animate([{ opacity: 1 }, { opacity: 0 }], { ...motion, fill: 'forwards' }).finished.finally(() => outgoing.remove()).catch(() => {});
    views.querySelector(':scope > section:not([hidden])').animate([{ opacity: 0, transform: 'translateY(6px)' }, { opacity: 1, transform: 'none' }], motion);
  }
}
function fit() {
  if (!native) return;
  const panel = root.querySelector('.flyout');
  // Measure the content before applying the monitor cap.
  const maxHeight = panel.style.maxHeight;
  panel.style.maxHeight = 'none';
  const height = Math.ceil(panel.scrollHeight);
  panel.style.maxHeight = maxHeight;
  const position = () => {
    if (!flyoutLayout) return;
    const visibleHeight = panel.getBoundingClientRect().height;
    root.style.top = `${flyoutLayout.topAnchored ? 12 : Math.max(12, Math.min(flyoutLayout.anchorBottom - visibleHeight, flyoutLayout.limit + 12 - visibleHeight))}px`;
  };
  position();
  if (height !== lastHeight) {
    lastHeight = height;
    // Older animation frames must not resize the window after a newer frame.
    fitRequest = fitRequest.then(async () => {
      const targetHeight = lastHeight;
      const layout = await invoke('resize_flyout', { height: targetHeight });
      if (layout?.limit > 0) {
        flyoutLayout = layout;
        panel.style.maxHeight = `${layout.limit}px`;
        document.querySelector('#views').classList.toggle('scroll', targetHeight > layout.limit);
        position();
      }
    }).catch(() => {});
  }
  return fitRequest;
}
function setMessage(id,open){const wrap=document.getElementById(`${id}-wrap`);wrap.classList.toggle('open',open);wrap.setAttribute('aria-hidden',String(!open));}
function render(){
  const views=document.querySelector('#views'), before=views.getBoundingClientRect().height;
  const previousView=views.querySelector(':scope > section:not([hidden])');
  const outgoing=previousView && previousView.id!==(settingsOpen?'settings':'network-view')?previousView.cloneNode(true):null;
  if(!native)language=state.languageMode==='system' ? systemLanguage : state.languageMode;
  const t=text[language], paused=state.pauseRemaining>0, inactive=paused || state.connection!=='connected' || !state.profile;
  document.documentElement.lang=language==='zh'?'zh-Hans':'en';
  if(state.accent){document.documentElement.style.setProperty('--accent',state.accent);document.documentElement.style.setProperty('--accent-ink',state.accentInk);document.documentElement.dataset.theme=state.dark?'dark':'light';}
  root.querySelector('.flyout').setAttribute('aria-label',t.app);
  root.querySelector('.header').classList.toggle('inactive',inactive);
  root.querySelector('.badge').classList.toggle('inactive',inactive);
  setContent('current-icon',icon(paused?'pause':['disconnected','unreadable'].includes(state.connection)?'wifi_off':'wifi_1'),true);
  const current=state.current || (state.connection==='unreadable'?t.unreadable:t.none);
  setText('current-name',current);document.querySelector('#current-name').title=current;
  const status=paused?`${t.paused} · ${t.remaining} ${Math.ceil(state.pauseRemaining/60)} ${t.minutes}`:connectionLabel(state,language);
  setText('current-status',status);document.querySelector('#current-status').title=status;
  setText('reconnect-label',t.reconnect);setText('mode',paused?t.paused:!state.profile?t.off:state.connection==='unreadable'?t.unreadable:state.connection==='connected'?t.watching:state.connection==='reconnecting'?t.retrying:t.waiting);
  setText('target-label',t.target);setText('target',state.profile || '—');document.querySelector('#target').title=state.profile;
  setText('last-label',t.last);
  const date=state.lastReconnect?new Date(state.lastReconnect*1000):null;
  setText('last',date?new Intl.DateTimeFormat(language==='zh'?'zh-CN':'en-GB',{hour:'2-digit',minute:'2-digit',second:'2-digit',hour12:false}).format(date):t.never);
  document.querySelector('#last').title=date?date.toLocaleString(language==='zh'?'zh-CN':'en-GB'):t.never;
  if(date)document.querySelector('#last').dateTime=date.toISOString();
  setText('setup',t.setup);setMessage('setup',!state.profile);
  setText('profile-label',t.profile);setText('profile-hint',t.profileHint);setText('save-profile',t.save);
  const profile=document.querySelector('#profile');
  if(document.activeElement!==profile && !profileInvalid)profile.value=state.profile;
  profile.setAttribute('aria-invalid',String(profileInvalid));
  const profileError=document.querySelector('#profile-error');setMessage('profile-error',profileInvalid);setText(profileError,t.profileError);
  setText('pause-for-label',t.pauseFor);setText('minutes-unit',t.minutes);setText('language-label',t.language);
  const minutes=document.querySelector('#pause-minutes');
  if(document.activeElement!==minutes && !pauseInvalid)minutes.value=state.pauseMinutes;
  minutes.setAttribute('aria-invalid',String(pauseInvalid));
  const pauseError=document.querySelector('#pause-error');setMessage('pause-error',pauseInvalid);setContent(pauseError,icon('error_circle',16)+t.pauseError,true);
  for(const button of root.querySelectorAll('[data-minutes]'))button.setAttribute('aria-pressed',String(Number(button.dataset.minutes)===state.pauseMinutes));
  for(const button of root.querySelectorAll('[data-language]')){if(button.dataset.language==='system')setText(button,t.system);button.setAttribute('aria-pressed',String(button.dataset.language===state.languageMode));}
  document.querySelector('#network-view').hidden=settingsOpen;document.querySelector('#settings').hidden=!settingsOpen;
  const gear=document.querySelector('#gear');gear.title=t.settings;gear.setAttribute('aria-label',t.settings);gear.setAttribute('aria-expanded',String(settingsOpen));
  const pause=document.querySelector('#pause');setContent(pause,`${icon(paused?'play':'pause')}<span>${paused?t.resume:pauseLabel(state.pauseMinutes,language)}</span>`,true);pause.classList.toggle('primary',paused);pause.setAttribute('aria-pressed',String(paused));
  const error=document.querySelector('#error');setMessage('error',!!(state.error||localError));if(state.error||localError)setText(error,localError || `${t.attention}. ${state.error}`);
  document.querySelector('#github').setAttribute('aria-label',t.github);
  animateView(before,outgoing);
  requestAnimationFrame(fit);
}
let pending=Promise.resolve();
function change(action){
  pending=pending.then(async()=>{localError='';try{state=native?await invoke('update',{action}):mockUpdate(state,action);if(native)language=state.language;}catch{localError=text[language].failed;}render();});
}
document.querySelector('#pause').addEventListener('click',()=>change({type:'togglePause'}));
document.querySelector('#gear').addEventListener('click',()=>{settingsOpen=!settingsOpen;render();});
const minutesInput=document.querySelector('#pause-minutes');
function savePause(){const minutes=parsePauseMinutes(minutesInput.value);pauseInvalid=minutes===null;if(!pauseInvalid&&minutes!==state.pauseMinutes)change({type:'setPauseMinutes',minutes});render();}
minutesInput.addEventListener('blur',savePause);
minutesInput.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();savePause();}});
for(const button of root.querySelectorAll('[data-minutes]'))button.addEventListener('click',()=>{pauseInvalid=false;minutesInput.value=button.dataset.minutes;change({type:'setPauseMinutes',minutes:Number(button.dataset.minutes)});});
for(const button of root.querySelectorAll('[data-language]'))button.addEventListener('click',()=>change({type:'setLanguage',language:button.dataset.language}));
function saveProfile(){const profile=document.querySelector('#profile').value;profileInvalid=!validProfile(profile);if(!profileInvalid)change({type:'setProfile',profile});render();}
document.querySelector('#save-profile').addEventListener('click',saveProfile);
document.querySelector('#profile').addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();saveProfile();}});
document.querySelector('#github').addEventListener('click',event=>{if(native){event.preventDefault();invoke('open_project').catch(()=>{localError=text[language].failed;render();});}});
document.addEventListener('contextmenu',event=>event.preventDefault());
document.addEventListener('dragstart',event=>event.preventDefault());
document.addEventListener('wheel',event=>{if(event.ctrlKey)event.preventDefault();},{passive:false});
document.addEventListener('keydown',event=>{if(event.key==='Escape'){if(native)invoke('dismiss_flyout');else if(reducedMotion.matches)root.hidden=true;else root.querySelector('.flyout').animate([{opacity:1,transform:'none'},{opacity:0,transform:'translateY(12px)'}],motion).finished.then(()=>root.hidden=true);}if((event.ctrlKey||event.metaKey)&&['+','-','=','0'].includes(event.key))event.preventDefault();});
if(native){await listen('state-changed',event=>{state=event.payload;language=state.language;render();});try{state=await invoke('snapshot');language=state.language;}catch{state.error=text[language].attention;}}
render();
await document.fonts.ready;
motionReady = true;
reducedMotion.addEventListener('change', () => { if (reducedMotion.matches) document.getAnimations().forEach(animation => animation.finish()); requestAnimationFrame(fit); });
if (native) {
  await fit();
  const originalStyle = await invoke('prepare_flyout', { height: root.querySelector('.flyout').scrollHeight });
  await new Promise(requestAnimationFrame);
  await new Promise(requestAnimationFrame);
  await invoke('present_flyout', { originalStyle });
  new ResizeObserver(fit).observe(root.querySelector('.flyout'));
} else if (!reducedMotion.matches) root.querySelector('.flyout').animate([{ opacity: 0, transform: 'translateY(12px)' }, { opacity: 1, transform: 'none' }], motion);
// Only the visible countdown ticks; native expiration belongs to the worker.
setInterval(()=>{if(state.pauseRemaining>0){state.pauseRemaining--;render();}},1000);
