const { invoke } = window.__TAURI__.core;
const appWindow = window.__TAURI__.window.getCurrentWindow();
let state, activeProfileId = "", installedApps = null, editingIndex = null, promptResolve = null, messageResolve = null;
const $ = s => document.querySelector(s);
const esc = (v="") => String(v).replace(/[&<>'"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;","'":"&#39;",'"':"&quot;"}[c]));
const profile = () => state.profiles.find(p=>p.id===activeProfileId)||state.profiles[0];
const persist = () => invoke("save_config",{config:state});

function theme(value){const dark=value==="dark"||(value==="system"&&matchMedia("(prefers-color-scheme: dark)").matches);document.documentElement.dataset.theme=dark?"dark":"light";document.querySelectorAll("[data-theme]").forEach(b=>b.classList.toggle("active",b.dataset.theme===value));}
function profiles(){
  $("#floatingMenu")?.remove();
  $("#profileList").innerHTML=state.profiles.map(p=>{const auto=state.auto_launch_enabled&&p.id===state.auto_launch_profile_id;return `<div class="profile-wrap ${auto?"has-auto":""}"><button class="profile ${p.id===activeProfileId?"active":""}" data-id="${p.id}"><span class="profile-name">${esc(p.name)}</span>${auto?'<span class="auto-mark" title="启动器打开后自动运行">▶</span>':''}</button><button class="profile-more" data-more="${p.id}" title="更多操作">•••</button></div>`}).join("");
  document.querySelectorAll(".profile").forEach(b=>b.onclick=()=>{activeProfileId=b.dataset.id;view("apps");render();});
  document.querySelectorAll("[data-more]").forEach(b=>b.onclick=e=>{e.stopPropagation();profileMenu(b.dataset.more,b)});
}
function apps(){
  const p=profile(),q=$("#appSearch").value.trim().toLowerCase();$("#profileTitle").textContent=p.name;$("#profileSummary").textContent=`${p.apps.length} 个应用 · ${p.apps.filter(a=>a.enabled!==false).length} 个已启用`;
  const rows=p.apps.map((app,index)=>({app,index})).filter(({app})=>!q||app.name.toLowerCase().includes(q)||app.path.toLowerCase().includes(q));
  $("#appList").innerHTML=rows.map(({app,index})=>`<div class="app-row"><div class="app-icon">${esc(app.name.slice(0,1).toUpperCase())}</div><div><div class="app-name">${esc(app.name)} ${app.run_as_admin?'<span class="admin-badge">管理员</span>':''}</div><div class="app-state">${app.enabled!==false?"准备就绪":"已暂停"}</div></div><div class="app-path" title="${esc(app.path)}">${esc(app.path)}</div><div class="row-actions"><label class="toggle"><input type="checkbox" data-toggle="${index}" ${app.enabled!==false?"checked":""}><i></i></label><button data-up="${index}" title="上移">↑</button><button data-down="${index}" title="下移">↓</button><button data-edit="${index}" title="编辑">✎</button><button data-delete="${index}" title="删除">×</button></div></div>`).join("");
  $("#emptyState").classList.toggle("hidden",p.apps.length>0);rowEvents();
}
function rowEvents(){
  document.querySelectorAll("[data-toggle]").forEach(e=>e.onchange=async()=>{profile().apps[+e.dataset.toggle].enabled=e.checked;await persist();apps();});
  document.querySelectorAll("[data-edit]").forEach(e=>e.onclick=()=>custom(+e.dataset.edit));
  document.querySelectorAll("[data-delete]").forEach(e=>e.onclick=async()=>{const i=+e.dataset.delete;if(await message("删除应用",`确定要从当前场景中删除“${profile().apps[i].name}”吗？`,{danger:true,confirm:true,confirmText:"删除"})){profile().apps.splice(i,1);await persist();apps();}});
  document.querySelectorAll("[data-up],[data-down]").forEach(e=>e.onclick=async()=>{const i=+(e.dataset.up??e.dataset.down),j=i+(e.dataset.up!==undefined?-1:1);if(j<0||j>=profile().apps.length)return;[profile().apps[i],profile().apps[j]]=[profile().apps[j],profile().apps[i]];await persist();apps();});
}
function render(){profiles();apps()}function view(v){$("#appsView").classList.toggle("active",v==="apps");$("#settingsView").classList.toggle("active",v==="settings");$("#settingsBtn").classList.toggle("active",v==="settings")}
function toast(m){const e=$("#toast");e.textContent=m;e.classList.add("show");clearTimeout(e.timer);e.timer=setTimeout(()=>e.classList.remove("show"),2500)}
function ask(title,text,value=""){ $("#promptTitle").textContent=title;$("#promptText").textContent=text;$("#promptInput").value=value;$("#promptBackdrop").classList.add("open");$("#promptInput").focus();return new Promise(r=>promptResolve=r)}
function askClose(v=null){$("#promptBackdrop").classList.remove("open");if(promptResolve){promptResolve(v);promptResolve=null}}
function message(title,text,{danger=false,confirm=false,confirmText="确认"}={}){$("#messageTitle").textContent=title;$("#messageText").textContent=String(text);$("#messageSymbol").textContent=danger?"!":"✓";$("#messageSymbol").classList.toggle("danger",danger);$("#messageCancel").style.display=confirm?"inline-flex":"none";$("#messageConfirm").textContent=confirmText;$("#messageBackdrop").classList.add("open");$("#messageConfirm").focus();return new Promise(r=>messageResolve=r)}
function messageClose(result){$("#messageBackdrop").classList.remove("open");if(messageResolve){messageResolve(result);messageResolve=null}}
function modal(mode){$("#modalBackdrop").classList.add("open");const installed=mode==="installed";$("#installedPanel").style.display=installed?"block":"none";$("#customForm").classList.toggle("visible",!installed);$("#modalTitle").textContent=installed?"选择已安装应用":editingIndex===null?"自定义添加":"编辑应用";$("#modalSubtitle").textContent=installed?"从开始菜单快捷方式中搜索并添加":"配置应用的启动信息"}
function modalClose(){$("#modalBackdrop").classList.remove("open");editingIndex=null}
async function installed(){editingIndex=null;modal("installed");if(installedApps)return installedRender();$("#installedLoading").style.display="block";$("#installedList").innerHTML="";try{installedApps=await invoke("installed_applications");installedRender()}catch(e){$("#installedLoading").textContent=`读取失败：${e}`}}
function installedRender(){if(!installedApps)return;$("#installedLoading").style.display="none";const q=$("#installedSearch").value.trim().toLowerCase(),list=installedApps.filter(a=>!q||a.name.toLowerCase().includes(q)||a.path.toLowerCase().includes(q));$("#installedList").innerHTML=list.map((a,i)=>`<button class="installed-item" data-i="${i}"><i>${esc(a.name.slice(0,1))}</i><b>${esc(a.name)}</b><small>${esc(a.path)}</small></button>`).join("");document.querySelectorAll(".installed-item").forEach((b,i)=>b.onclick=async()=>{profile().apps.push(list[i]);await persist();modalClose();apps();toast(`已添加 ${list[i].name}`)})}
function custom(index=null){editingIndex=index;const f=$("#customForm");f.reset();f.enabled.checked=true;f.runAsAdmin.checked=false;if(index!==null){const a=profile().apps[index];f.name.value=a.name;f.path.value=a.path;f.args.value=a.args||"";f.workingDir.value=a.working_dir||"";f.enabled.checked=a.enabled!==false;f.runAsAdmin.checked=a.run_as_admin===true}modal("custom")}
async function launch(auto=false){const list=profile().apps.filter(a=>a.enabled!==false);if(!list.length){if(!auto)toast("当前场景没有已启用的应用");return}const errors=await invoke("launch_apps",{apps:list});if(errors.length)await message("部分应用启动失败",errors.join("\n"),{danger:true});else if(!auto)toast(`已启动 ${list.length} 个应用`)}
function profileMenu(profileId,anchor){
  const old=$("#floatingMenu");if(old)old.remove();const target=state.profiles.find(p=>p.id===profileId);if(!target)return;
  const isAuto=state.auto_launch_enabled&&state.auto_launch_profile_id===profileId,m=document.createElement("div");m.id="floatingMenu";m.className="scene-menu";
  m.innerHTML=`<button id="renameP">✎　重命名</button><button id="toggleAutoP">${isAuto?"◇　取消自启动":"▶　设为自启动"}</button><div class="menu-separator"></div><button id="deleteP" class="danger">×　删除</button>`;document.body.appendChild(m);
  const rect=anchor.getBoundingClientRect();m.style.left=`${Math.min(rect.right+7,innerWidth-184)}px`;m.style.top=`${Math.min(rect.top,innerHeight-m.offsetHeight-12)}px`;
  m.onclick=e=>e.stopPropagation();
  $("#renameP").onclick=async()=>{m.remove();const n=await ask("重命名场景","输入新的场景名称",target.name);if(n?.trim()){if(state.profiles.some(p=>p.id!==target.id&&p.name===n.trim()))return toast("该名称已经存在");target.name=n.trim();await persist();render()}};
  $("#toggleAutoP").onclick=async()=>{m.remove();if(isAuto){state.auto_launch_enabled=false;state.auto_launch_profile_id="";toast("已取消场景自启动")}else{state.auto_launch_enabled=true;state.auto_launch_profile_id=target.id;toast(`已设为自启动：${target.name}`)}await persist();render()};
  $("#deleteP").onclick=async()=>{m.remove();if(state.profiles.length===1)return toast("至少保留一个启动场景");if(await message("删除启动场景",`确定要删除“${target.name}”及其中的全部应用配置吗？`,{danger:true,confirm:true,confirmText:"删除"})){state.profiles=state.profiles.filter(p=>p.id!==target.id);if(state.auto_launch_profile_id===target.id){state.auto_launch_enabled=false;state.auto_launch_profile_id=""}if(activeProfileId===target.id)activeProfileId=state.profiles[0].id;await persist();render()}};
}

$("#minimize").onclick=()=>appWindow.minimize();$("#maximize").onclick=()=>appWindow.toggleMaximize();$("#close").onclick=()=>appWindow.close();$("#settingsBtn").onclick=()=>view("settings");$("#launchAll").onclick=()=>launch();$("#appSearch").oninput=apps;
$("#installedAdd").onclick=installed;$("#emptyAdd").onclick=installed;$("#customAdd").onclick=()=>custom();$("#installedSearch").oninput=installedRender;document.querySelectorAll(".modal-close,.modal-cancel").forEach(b=>b.onclick=modalClose);
$("#addProfile").onclick=async()=>{const n=await ask("新建启动场景","例如：工作、直播、设计或游戏");if(!n?.trim())return;if(state.profiles.some(p=>p.name===n.trim()))return toast("该名称已经存在");const p={id:crypto.randomUUID().replaceAll("-",""),name:n.trim(),apps:[]};state.profiles.push(p);activeProfileId=p.id;await persist();view("apps");render()};
$("#promptForm").onsubmit=e=>{e.preventDefault();askClose($("#promptInput").value)};$("#promptCancel").onclick=()=>askClose();
$("#messageConfirm").onclick=()=>messageClose(true);$("#messageCancel").onclick=()=>messageClose(false);
$("#browsePath").onclick=async()=>{const path=await invoke("browse_executable");if(path){const f=$("#customForm");f.path.value=path;if(!f.name.value)f.name.value=path.split(/[\\/]/).pop().replace(/\.[^.]+$/,"")}};
$("#customForm").onsubmit=async e=>{e.preventDefault();const f=e.currentTarget,a={name:f.name.value.trim(),path:f.path.value.trim(),args:f.args.value.trim(),working_dir:f.workingDir.value.trim(),enabled:f.enabled.checked,run_as_admin:f.runAsAdmin.checked};if(editingIndex===null)profile().apps.push(a);else profile().apps[editingIndex]=a;await persist();modalClose();apps();toast("应用配置已保存")};
$("#startupToggle").onchange=async e=>{try{await invoke("set_startup",{enabled:e.target.checked});toast(e.target.checked?"已开启开机自启动":"已关闭开机自启动")}catch(err){e.target.checked=!e.target.checked;await message("设置失败",err,{danger:true})}};
document.querySelectorAll("[data-theme]").forEach(b=>b.onclick=async()=>{state.theme=b.dataset.theme;theme(state.theme);await persist()});matchMedia("(prefers-color-scheme: dark)").onchange=()=>{if(state?.theme==="system")theme("system")};

document.addEventListener("click",()=>$("#floatingMenu")?.remove());
async function init(){state=await invoke("load_config");state.theme||="system";activeProfileId=state.profiles[0].id;theme(state.theme);render();$("#startupToggle").checked=await invoke("startup_enabled");if(state.auto_launch_enabled){const p=state.profiles.find(p=>p.id===state.auto_launch_profile_id);if(p){activeProfileId=p.id;render();setTimeout(()=>launch(true),500)}else{state.auto_launch_enabled=false;state.auto_launch_profile_id="";await persist();render()}}}
init().catch(e=>message("应用初始化失败",e,{danger:true}));
