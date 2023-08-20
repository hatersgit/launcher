import { useState, useEffect } from "react";
import {open} from '@tauri-apps/api/dialog';
import { desktopDir, homeDir } from "@tauri-apps/api/path";
import { invoke } from "@tauri-apps/api/tauri";
import { error, info } from "tauri-plugin-log-api";
import { listen } from '@tauri-apps/api/event';
import { RaceBy } from '@uiball/loaders'
import "./App.css";

class FileStat {
  name: string;
  date: string;

  constructor(name: string, date: string) {
    this.name = name;
    this.date = date;
  }
}

class Settings {
  wowdir: string;
  files: FileStat[];

  constructor() {
    this.wowdir = ".";
    this.files = [new FileStat("patch-I.mpq", "."), new FileStat("ForgedWoWCommunication.zip", ".")];
  }

  update(json: any) {
    this.wowdir = json.wowDir;
    for(var i = 0; i < this.files.length; i++) {
      let entry = this.files[i];
      entry.name = json.files[i].name;
      entry.date = json.files[i].date;
    }
  }
}

var G_SETTINGS = new Settings();
var firstRun = false;

function App() {
  const realmlistPath = '\\Data\\enUS\\realmlist.wtf';
  const realmlist = 'SET realmList 141.94.242.52';

  const actionPhases = {
    locate: 'LOCATE',
    update: 'VERIFYING',
    updating: 'PATCHING',
    play: 'PLAY'
  }

  const settingsDir = 'Echo Launcher\\';
  const settingsFile = 'launcher.settings.json';
  const [actionPhase, setActionPhase] = useState(actionPhases.locate);
  const [patchStatus, setPatchStatus] = useState("Not running")
  const [settingsContext, setSettingsContext] = useState(false);
  const [running, setRunning] = useState(false);

  async function openDiscord() {
    openurl('https://discord.gg/SArh4ngaHp');
  }

  async function openChangelog() {
    openurl('https://discord.com/channels/1126325498612547606/1139691709610078258');
  }

  async function openurl(url: string) {
    window.open(url, '_blank')!.focus;
  }

  interface ProgressPayload {
    message: string;
  }

  async function findWorkingDir() {
    try {
      let home = await homeDir();
      let created = await tryInitSettingsFile(home)
      info('Settings Created: '+created)
      if (created) {
        var settingsString: string = await invoke('read_settings', {path: home+settingsDir+settingsFile});
        info("READ: "+settingsString)
        var obj = JSON.parse(settingsString);
        if (obj.wowDir === ".") {
          info("Starter dir found, requesting corrected dir.")
          setActionPhase(actionPhases.locate)
        } else {
          setActionPhase(actionPhases.update);
          G_SETTINGS.update(obj);
          await compareFiles();
        }
      }
    } catch (e) {
        error("ERROR: "+e)
    }
  };
  
  async function compareFiles() {
    if (!running) {
      setRunning(true)
      setActionPhase(actionPhases.updating);
      while(true) {
        let running: Boolean = await invoke('check_wow_running');
        if (!running) {
          break;
        }
        setPatchStatus("WoW is currently running, close the game to continue!");
      }

      setPatchStatus("");
      const { appWindow } = await import("@tauri-apps/api/window");
      var done: string = await invoke('check_file_version_and_download', {payload: serializeSettings(G_SETTINGS), window: appWindow});
      info(done);
      var obj = JSON.parse(done);
      G_SETTINGS.update(obj);
      saveSetting(G_SETTINGS);
      setActionPhase(actionPhases.play);
  
      setRunning(false);
    }
  }
  
  async function saveSetting(settings: Settings) {
    info('Resolving correctness of settings.')
    let wow_dir = settings.wowdir;
    const isWowDir = await invoke('exists' ,{dir: wow_dir+'\\WoW.exe'}) 
      || await invoke('exists' ,{dir: wow_dir+'\\wow.exe'}) || await invoke('exists' ,{dir: wow_dir+'\\Wow.exe'}) 
      || await invoke('exists' ,{dir: wow_dir+'\\Interface\\Addons'}) || await invoke('exists' ,{dir: wow_dir+'\\Data'});
    
    let settingsContent = await invoke('read_settings', {path: await homeDir()+settingsDir+settingsFile});
    let newSettings = serializeSettings(settings);
    info('Wow dir valid: '+isWowDir)
    let updated = settingsContent !== newSettings;
  
    if (isWowDir && updated) {
      try {
        info("Trying to set")
        G_SETTINGS = settings;
        await invoke ('create_file', {path: await homeDir()+settingsDir+settingsFile, content: newSettings})
        await findWorkingDir()
      } catch (e) {
          console.log(e);
      }
    }
  };
  
  async function tryInitSettingsFile (file: string) {
    try {
      let fileMade = await invoke('exists', {dir: file+settingsDir+settingsFile});
      if (!fileMade) {
        info('Could not find settings file: creating one')
        let dirMade = await invoke('exists', {dir: file+settingsDir});
        if (!dirMade) {
          await invoke('create_dir', {path: file+settingsDir});
        }
        let filename = file+settingsDir+settingsFile;
        let fileWrite = await invoke('create_file', {path: filename, content: serializeSettings(G_SETTINGS)});
        return fileWrite;
      }
      return fileMade;
    } catch (e) {
        console.log(e);
    }
  };
  
  function serializeSettings (settings: Settings) {
    let jsonFormat = `{"wowDir":"${settings.wowdir.split('\\').join('\\\\')}","files":[${serializeFiles(settings.files)}]}`;
    return jsonFormat;
  }
  
  function serializeFiles (files: Array<FileStat>) {
    let out = ""
    files.forEach(function (value, index) {
      out += `{"name":"${value.name}","date":"${value.date}"}`
      if (index !== files.length-1){
        out += ","
      }
    })
    return out
  }

  async function selectFile() {
    const selected = await open({
        directory: true,
        multiple: false, 
        defaultPath: await desktopDir()
    }) as string;
    info('selected: '+selected)
    if (selected !== null) {
      let settings = new Settings();
      settings.wowdir = selected;
      await saveSetting(settings);
    }
};

  async function startExecutable() {
    let running: Boolean = await invoke('check_wow_running');
    if (!running) {
      let setRealm = await invoke('set_realmlist', {realm_path: G_SETTINGS.wowdir+realmlistPath, realm_info: realmlist});
      info(""+setRealm)
      let wowExe = G_SETTINGS.wowdir+"\\wow.exe";
      info("Attempting to start: "+wowExe);
      invoke('start_wow', {wow_exe: wowExe});
    } 
  }

  async function handleAction() {
    switch (actionPhase) {
      case actionPhases.locate:
        await selectFile();
        break;
      case actionPhases.update:
        break;
      case actionPhases.play:
        await startExecutable();
        break;
      case actionPhases.updating:
        break;
    }
  }

  function contextMenu () {
    if (actionPhase === actionPhases.play) {
      setSettingsContext(!settingsContext);
    }
  }

  useEffect(() => {
    const progListener = listen<ProgressPayload>("prog", ({payload}) => {
      const {message} = payload;
      setPatchStatus(message);
    });

    return () => {
      progListener.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (!firstRun) {
      firstRun = true;
      findWorkingDir();
    }
  }, []);

  return (
    <div className='container' >
      {actionPhase === actionPhases.updating ? <div className='patching-focus'/>
          : <></> }
      <div className='menubar'>
        <button className='menubutton' onClick={openDiscord}>discord</button>
        <button className='menubutton' onClick={openChangelog}>changelog</button>
      </div>
      <div className='actions'>
        <button className='actionbutton' onClick={handleAction}>{actionPhase}</button>
        {actionPhase === actionPhases.updating ?
          <div>
            <div className="patch-text"> {patchStatus}</div>
            <RaceBy size={340.38} color="#ffffff"/>
          </div>
          : <></> }
        {actionPhase === actionPhases.play ? <div className="settings">
          <button className='subaction' onClick={contextMenu}>
            settings
          </button>
          {settingsContext ? <div className="settings-subdir">
            <button className="subdirbutton">open addons</button> 
            <button className="subdirbutton" onClick={selectFile}>locate wow</button>
          </div>
          : <></> }
        </div>
          : <></>}
      </div>
    </div>
  );
}

export default App;