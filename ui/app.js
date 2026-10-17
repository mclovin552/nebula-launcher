const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;

const ICONS = {
  home: '<path d="M3 11l9-8 9 8v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
  box: '<path d="M21 8l-9-5-9 5v8l9 5 9-5z"/><path d="M3 8l9 5 9-5M12 13v8"/>',
  compass: '<circle cx="12" cy="12" r="9"/><path d="M15.5 8.5l-2 5-5 2 2-5z"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21c0-4 4-6 8-6s8 2 8 6"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>',
  term: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>',
  play: '<path d="M6 4l14 8-14 8z" fill="currentColor"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor"/>',
  folder: '<path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  trash: '<path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13"/>',
  edit: '<path d="M4 20h4L19 9l-4-4L4 16z"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  x: '<path d="M6 6l12 12M18 6L6 18"/>',
  down: '<path d="M12 4v12M6 11l6 6 6-6M5 20h14"/>',
  list: '<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
  copy: '<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a1 1 0 0 1 1-1h10"/>',
  up: '<path d="M12 20V8M6 13l6-6 6 6M5 4h14"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7"/>',
  server: '<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>'
};

const ic = (n) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[n] || ''}</svg>`;

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

const NAV = [
  ['home', 'Inicio', 'home'],
  ['instances', 'Instancias', 'box'],
  ['discover', 'Descubrir', 'compass'],
  ['accounts', 'Cuentas', 'user'],
  ['console', 'Consola', 'term'],
  ['settings', 'Ajustes', 'gear']
];

const KINDS = [
  ['mod', 'Mods'],
  ['modpack', 'Modpacks'],
  ['resourcepack', 'Texturas'],
  ['shader', 'Shaders']
];

const CONTENT_TABS = [
  ['mods', 'Mods'],
  ['resourcepacks', 'Texturas'],
  ['shaderpacks', 'Shaders'],
  ['saves', 'Mundos'],
  ['servers', 'Servidores'],
  ['backups', 'Copias'],
  ['screens', 'Capturas']
];

const POPULAR_SERVERS = [
  ['Hypixel', 'hypixel.net'],
  ['CubeCraft', 'mco.cubecraft.net'],
  ['BlocksMC', 'blocksmc.com'],
  ['MineVille', 'play.mineville.org'],
  ['Purple Prison', 'purpleprison.net'],
  ['Minewind', 'minewind.com'],
  ['2b2t', '2b2t.org']
];

const JVM_PRESETS = {
  'Preset vanilla (estable)': '',
  'Preset rendimiento (G1GC)': '-XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1MaxNewSizePercent=40 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M',
  'Preset memoria baja': '-Xms512M -XX:MaxHeapFreeRatio=30 -XX:MinHeapFreeRatio=10 -XX:+UseSerialGC'
};

const LOADERS = [
  ['vanilla', 'Vanilla'],
  ['fabric', 'Fabric'],
  ['quilt', 'Quilt'],
  ['forge', 'Forge'],
  ['neoforge', 'NeoForge']
];

const S = {
  view: 'home',
  instances: [],
  accounts: [],
  selectedAccount: null,
  activeInstance: null,
  versions: null,
  settings: null,
  sys: null,
  running: new Set(),
  logs: [],
  news: null,
  progress: null,
  loginActive: false,
  discover: { kind: 'mod', query: '', sort: 'relevance', offset: 0, items: [], total: 0, loading: false, target: '', error: '' },
  content: { id: null, tab: 'mods', items: [], updates: null, open: false },
  editAccent: 0
};

const $ = (s, r = document) => r.querySelector(s);

const EN = {
  'Inicio': 'Home', 'Instancias': 'Instances', 'Descubrir': 'Discover', 'Cuentas': 'Accounts', 'Consola': 'Console', 'Ajustes': 'Settings',
  'Jugar': 'Play', 'Detener': 'Stop', 'Crear instancia': 'Create instance', 'Nueva instancia': 'New instance', 'Tu primera instancia': 'Your first instance',
  'Elige una versión y un cargador para empezar a jugar.': 'Pick a version and a loader to start playing.',
  'Instancias recientes': 'Recent instances', 'Ver todas': 'View all', 'Novedades de Minecraft': 'Minecraft news', 'Cargando novedades…': 'Loading news…',
  'No se pudieron cargar las novedades. Comprueba tu conexión.': 'Could not load the news. Check your connection.',
  'Aún no tienes instancias': 'You have no instances yet', 'Cada instancia tiene sus propios mods, mundos y ajustes.': 'Each instance has its own mods, worlds and settings.',
  'Contenido': 'Content', 'Abrir carpeta': 'Open folder', 'Editar': 'Edit', 'Eliminar': 'Delete', 'Duplicar': 'Duplicate', 'Exportar modpack': 'Export modpack',
  'Importar modpack': 'Import modpack', 'Sin grupo': 'Ungrouped', 'Tiempo jugado': 'Play time',
  'Mods': 'Mods', 'Modpacks': 'Modpacks', 'Texturas': 'Textures', 'Shaders': 'Shaders', 'Mundos': 'Worlds', 'Servidores': 'Servers', 'Copias': 'Backups',
  'Buscar en': 'Search in', 'Relevancia': 'Relevance', 'Descargas': 'Downloads', 'Más nuevos': 'Newest', 'Actualizados': 'Recently updated', 'Sin instancias': 'No instances',
  'Instancia de destino': 'Target instance', 'Ordenar': 'Sort by', 'Sin resultados. Prueba con otra búsqueda.': 'No results. Try another search.', 'Buscando…': 'Searching…',
  'descargas': 'downloads', 'Instalar como instancia': 'Install as instance', 'Instalar': 'Install', 'Cargar más': 'Load more', 'Cargando…': 'Loading…',
  'Inicia sesión con tu cuenta que posee Minecraft Java para jugar en servidores oficiales.': 'Sign in with the account that owns Minecraft Java to play on official servers.',
  'Iniciar sesión con Microsoft': 'Sign in with Microsoft', 'Sin conexión': 'Offline', 'Cuenta Microsoft': 'Microsoft account',
  'Juega en modo individual o en servidores sin verificación con cualquier nombre.': 'Play singleplayer or on unverified servers with any name.',
  'Nombre de jugador': 'Player name', 'Añadir': 'Add', 'Tus cuentas': 'Your accounts', 'Todavía no hay cuentas añadidas.': 'No accounts added yet.', 'En uso': 'In use', 'Usar': 'Use',
  'Quitar cuenta': 'Remove account', 'Cambiar skin': 'Change skin', 'Skin': 'Skin', 'Clásica': 'Classic', 'Delgada': 'Slim', 'Elegir archivo PNG': 'Choose PNG file', 'Modelo': 'Model',
  'Limpiar': 'Clear', 'Guardar cambios': 'Save changes', 'Abrir carpeta de datos': 'Open data folder', 'Memoria': 'Memory', 'Máxima': 'Maximum', 'Mínima': 'Minimum', 'de': 'of',
  'Java y juego': 'Java and game', 'Ruta de Java (vacío = automática, se descarga si hace falta)': 'Java path (empty = automatic, downloaded if needed)', 'Argumentos de JVM': 'JVM arguments',
  'Comando envoltorio (por ejemplo gamemoderun o mangohud)': 'Wrapper command (for example gamemoderun or mangohud)', 'Ancho de ventana': 'Window width', 'Alto de ventana': 'Window height',
  'Descargas simultáneas': 'Simultaneous downloads', 'Linux': 'Linux', 'Variables de entorno (una por línea, CLAVE=valor)': 'Environment variables (one per line, KEY=value)',
  'Comando antes de jugar': 'Command before playing', 'Comando al cerrar el juego': 'Command after the game closes', 'Idioma': 'Language', 'Repositorio de actualizaciones (usuario/repo en GitHub)': 'Update repository (user/repo on GitHub)',
  'Cuentas y servicios': 'Accounts and services', 'Ajustes guardados': 'Settings saved',
  'Cancelar': 'Cancel', 'Crear': 'Create', 'Creando…': 'Creating…', 'Guardar': 'Save', 'Cerrar': 'Close', 'Nombre': 'Name', 'Cargador': 'Loader', 'Versión de Minecraft': 'Minecraft version', 'Versión del cargador': 'Loader version',
  'No aplica': 'Not applicable', 'Mostrar snapshots y versiones antiguas': 'Show snapshots and old versions', '(última)': '(latest)', 'No disponible para esta versión': 'Not available for this version', 'Error de red': 'Network error',
  'Escribe un nombre para la instancia': 'Enter a name for the instance', 'Instancia creada': 'Instance created', 'Editar instancia': 'Edit instance', 'Grupo': 'Group', 'Color': 'Color',
  'Memoria máxima en MB (vacío = global)': 'Maximum memory in MB (empty = global)', 'Ruta de Java (vacío = global)': 'Java path (empty = global)', 'Argumentos de JVM adicionales': 'Extra JVM arguments',
  'Variables de entorno de la instancia': 'Instance environment variables', 'Eliminar instancia': 'Delete instance', 'No se puede deshacer.': 'This cannot be undone.',
  'Se borrarán': 'This will delete', 'sus mundos y su configuración.': 'its worlds and its settings.',
  'Desactivar': 'Disable', 'Activar': 'Enable', 'desactivado': 'disabled', 'Carpeta': 'Folder', 'No hay nada aquí todavía. Búscalo en Descubrir.': 'Nothing here yet. Find it in Discover.',
  'Añadir archivo': 'Add file', 'Buscar actualizaciones': 'Check for updates', 'Actualizar': 'Update', 'Actualizar todo': 'Update all', 'Todo está al día': 'Everything is up to date',
  'actualizaciones disponibles': 'updates available', 'Crear copia ahora': 'Back up now', 'Restaurar': 'Restore', 'Copia creada': 'Backup created', 'Copia restaurada': 'Backup restored',
  'Las copias guardan todos los mundos de la instancia.': 'Backups store every world of the instance.', 'Arrastra archivos aquí para añadirlos': 'Drop files here to add them',
  'Dirección del servidor': 'Server address', 'Nombre del servidor': 'Server name', 'Aún no hay servidores. Añade uno para entrar directo al iniciar.': 'No servers yet. Add one to join directly on launch.',
  'Entrar': 'Join', 'Servidor añadido': 'Server added',
  'Iniciar sesión con Microsoft': 'Sign in with Microsoft', 'Abre la página de Microsoft e introduce este código:': 'Open the Microsoft page and enter this code:',
  'Abrir página de inicio de sesión': 'Open sign-in page', 'Esperando a que completes el inicio de sesión…': 'Waiting for you to finish signing in…',
  'Sesión iniciada correctamente': 'Signed in successfully', 'No se pudo iniciar sesión': 'Could not sign in',
  'Añade una cuenta para jugar': 'Add an account to play', 'Preparando…': 'Preparing…', 'Cancelar descarga': 'Cancel download',
  'Minecraft se cerró con un error': 'Minecraft closed with an error', 'Minecraft se cerró con un error. Revisa la consola.': 'Minecraft closed with an error. Check the console.',
  'El juego se cerró con un error': 'The game closed with an error', 'Posibles causas': 'Possible causes', 'Copiar registro': 'Copy log', 'Registro copiado': 'Log copied',
  'No se encontró un patrón conocido. Copia el registro y compártelo para pedir ayuda.': 'No known pattern found. Copy the log and share it to ask for help.',
  'Falta memoria. Sube la memoria máxima en Ajustes o en la instancia.': 'Out of memory. Raise the maximum memory in Settings or in the instance.',
  'La versión de Java no coincide. Deja la ruta de Java vacía para que Nebula use la correcta.': 'Wrong Java version. Leave the Java path empty so Nebula uses the right one.',
  'Un mod no es compatible con esta versión del juego o con otro mod. Desactiva los mods recientes e inténtalo de nuevo.': 'A mod is incompatible with this game version or another mod. Disable recent mods and try again.',
  'Falta una dependencia. Instala la librería que pide el mod, como Fabric API.': 'A dependency is missing. Install the library the mod asks for, such as Fabric API.',
  'Problema gráfico. Actualiza los drivers, desactiva los shaders o prueba otra variable de entorno OpenGL.': 'Graphics problem. Update your drivers, disable shaders or try another OpenGL environment variable.',
  'El sistema cerró el juego por falta de RAM. Baja la memoria asignada o cierra otras aplicaciones.': 'The system killed the game for lack of RAM. Lower the assigned memory or close other apps.',
  'Hay una nueva versión de Nebula Launcher': 'A new version of Nebula Launcher is available', 'Descargar': 'Download',
  'Novedades': 'News', 'Modpack': 'Modpack', 'Instalado': 'Installed', 'Modpack instalado como instancia': 'Modpack installed as an instance', 'Crea o elige una instancia de destino': 'Create or pick a target instance',
  'Exportado': 'Exported', 'Importar': 'Import', 'dependencia(s)': 'dependency(ies)', 'Instalado con': 'Installed with',
  'Descargando archivos del juego': 'Downloading game files', 'Preparando versión': 'Preparing version', 'Comprobando cuenta': 'Checking account', 'Leyendo índice de recursos': 'Reading asset index',
  'Preparando Java': 'Preparing Java', 'Iniciando Minecraft': 'Starting Minecraft', 'Minecraft en ejecución': 'Minecraft is running', 'Descargando Java': 'Downloading Java',
  'Instalando Forge': 'Installing Forge', 'Instalando NeoForge': 'Installing NeoForge', 'Buscando versión del modpack': 'Looking up modpack version', 'Descargando modpack': 'Downloading modpack',
  'Creando instancia': 'Creating instance', 'Descargando contenido del modpack': 'Downloading modpack content', 'Modpack instalado': 'Modpack installed',
  'Español': 'Spanish', 'Inglés': 'English', 'Añadir cuenta': 'Add account',
  'Optimizar': 'Optimize', 'Optimizando…': 'Optimizing…', 'Optimizando instancia': 'Optimizing instance', 'Instancia optimizada': 'Instance optimized',
  'mod(s) de rendimiento': 'performance mod(s)', 'Detalles': 'Details', 'Galería': 'Gallery', 'Descripción': 'Description',
  'Verificar y reparar': 'Verify and repair', 'Verificando archivos': 'Verifying files', 'Reparando archivos': 'Repairing files',
  'archivos reparados': 'files repaired', 'archivos verificados, todo en orden': 'files verified, all good',
  'Cambiar icono (PNG)': 'Change icon (PNG)', 'Quitar icono': 'Remove icon', 'Icono actualizado': 'Icon updated',
  'Importar instancia': 'Import instance', 'Importando instancia…': 'Importing instance…', 'importada': 'imported',
  'Actualizar estado': 'Refresh status', 'Consultando…': 'Pinging…',
  'Minimizar a la bandeja del sistema al cerrar la ventana': 'Minimize to the system tray when closing the window',
  'favoritos': 'favorites',
  'Skins y capas': 'Skins and capes', 'Aplicar': 'Apply', 'Poner': 'Put on', 'Quitar': 'Remove', 'ACTIVA': 'ACTIVE',
  'Capa activada': 'Cape enabled', 'Capa quitada': 'Cape removed', 'Nombre': 'Name', 'opcional': 'optional',
  'Skin guardada en el armario': 'Skin saved to the wardrobe', 'Skin aplicada a tu cuenta': 'Skin applied to your account',
  'Servidores populares': 'Popular servers', 'Actualizar modpack': 'Update modpack', 'Modpack actualizado': 'Modpack updated',
  'Usar como icono': 'Use as icon', 'Captura puesta como icono de la instancia': 'Screenshot set as instance icon',
  'Añadir': 'Add', 'Preset colocado; pulsa Guardar para aplicarlo': 'Preset entered; press Save to apply',
  'Esta instancia no guarda el modpack de Modrinth de origen': 'This instance does not store its source Modrinth modpack',
  'Ese servidor ya está en la lista': 'That server is already in the list',
  'Acerca de Nebula': 'About Nebula', 'Modpack de Modrinth actualizado': 'Modrinth modpack updated',
  'Capas de tu cuenta': 'Your account capes', 'Capa': 'Cape',
  'Poner skin': 'Apply skin', 'Elegir capa (PNG)': 'Choose cape (PNG)', 'Skin local': 'Local skin',
  'Instancia': 'Instance'
};
const EN_KEYS = Object.keys(EN).sort((a, b) => b.length - a.length);
function tr(str) {
  if (!S.settings || S.settings.language !== 'en') return str;
  const stash = [];
  let out = String(str).replace(/data:[a-z]+\/[a-z]+;base64,[A-Za-z0-9+/=]+/g, (m) => {
    stash.push(m);
    return `\u0001${stash.length}\u0001`;
  });
  for (const k of EN_KEYS) if (out.includes(k)) out = out.split(k).join(EN[k]);
  return out.replace(/\u0001(\d+)\u0001/g, (_, n) => stash[Number(n) - 1] || '');
}

async function api(cmd, args = {}, quiet = false) {
  try {
    return await invoke(cmd, args);
  } catch (e) {
    if (!quiet) toast(typeof e === 'string' ? e : (e && e.message) || 'Error inesperado', 'err');
    throw e;
  }
}

function toast(msg, kind = '') {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = tr(msg);
  $('#toasts').appendChild(el);
  setTimeout(() => el.remove(), kind === 'err' ? 8000 : 4000);
}

function viewTheme() {
  const t = S.settings && S.settings.theme;
  const dark = !t || t === 'dark';
  return dark ? 'dark' : t;
}

function applyTheme() {
  document.body.classList.toggle('light', viewTheme() === 'light');
  document.body.classList.toggle('contrast', viewTheme() === 'contrast');
}

let AC = null;
function beep(freq = 520, dur = 0.12, type = 'sine', vol = 0.05) {
  if (localStorage.getItem('nebula-sound') === '0') return;
  try {
    AC = AC || new (window.AudioContext || window.webkitAudioContext)();
    const o = AC.createOscillator();
    const g = AC.createGain();
    o.type = type;
    o.frequency.value = freq;
    g.gain.value = vol;
    o.connect(g);
    g.connect(AC.destination);
    o.start();
    o.stop(AC.currentTime + dur);
  } catch (_) {}
}
function playSound(name) {
  if (name === 'start') { beep(392, 0.1); setTimeout(() => beep(523, 0.14), 110); }
  else if (name === 'exit') { beep(330, 0.14, 'sine', 0.04); setTimeout(() => beep(262, 0.2), 130); }
  else if (name === 'done') beep(660, 0.18, 'triangle', 0.04);
  else if (name === 'err') beep(196, 0.25, 'square', 0.03);
}

function fmtNum(n) {
  n = Number(n) || 0;
  if (n >= 1e9) return (n / 1e9).toFixed(1) + ' mil M';
  if (n >= 1e6) return (n / 1e6).toFixed(1) + ' M';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + ' mil';
  return String(n);
}

function fmtSize(b) {
  if (b >= 1048576) return (b / 1048576).toFixed(1) + ' MB';
  if (b >= 1024) return Math.round(b / 1024) + ' KB';
  return b + ' B';
}

const LOADER_NAME = { vanilla: 'Vanilla', fabric: 'Fabric', quilt: 'Quilt', forge: 'Forge', neoforge: 'NeoForge' };
const activeAccount = () => S.accounts.find((a) => a.id === S.selectedAccount) || null;
const currentInstance = () => S.instances.find((i) => i.id === S.activeInstance) || S.instances[0] || null;
const headUrl = (a) => `https://mc-heads.net/avatar/${encodeURIComponent(a.kind === 'microsoft' ? a.uuid : a.name)}/64`;

function renderRail() {
  $('#rail').innerHTML =
    `<div class="brand"><svg viewBox="0 0 32 32" width="30" height="30"><rect x="3" y="18" width="6" height="11" fill="#a78bfa"/><rect x="10" y="12" width="6" height="17" fill="#5f7fb8"/><rect x="17" y="7" width="6" height="22" fill="#ffc857"/><rect x="24" y="14" width="5" height="15" fill="#5f7fb8"/></svg></div>` +
    NAV.map(([id, label, icon]) => `<button class="rail-btn ${S.view === id ? 'active' : ''}" data-act="nav" data-view="${id}" aria-label="${tr(label)}">${ic(icon)}<span>${tr(label)}</span></button>`).join('');
}

function renderTop() {
  const nav = NAV.find((n) => n[0] === S.view);
  $('#crumb').textContent = nav ? tr(nav[1]) : '';
  const a = activeAccount();
  $('#account-chip').innerHTML = a
    ? `<img src="${headUrl(a)}" alt=""><span>${esc(a.name)}</span>`
    : `<span>${tr('Añadir cuenta')}</span>`;
  const up = S.updateAv;
  const old = document.getElementById('update-pill');
  if (old) old.remove();
  if (up && up.available && up.version) {
    const el = document.createElement('button');
    el.id = 'update-pill';
    el.dataset.act = 'update-go';
    el.dataset.url = up.url || 'https://github.com/mclovin552/nebula-launcher/releases/latest';
    el.title = 'Ir a descargar la versión ' + up.version;
    el.innerHTML = `<b class="dot">⬆</b><span>Nueva versión <b>${esc(up.version)}</b></span><b class="go">${tr('Actualizar')}</b>`;
    $('#topbar').appendChild(el);
  }
}

function setView(v) {
  S.view = v;
  if (v === 'discover' && !S.instances.some((i) => i.id === S.discover.target)) S.discover.target = S.activeInstance || '';
  renderRail();
  renderTop();
  renderView();
  if (v === 'discover') runSearch(true);
}

function renderView() {
  const el = $('#view');
  const r = { home: viewHome, instances: viewInstances, discover: viewDiscover, accounts: viewAccounts, console: viewConsole, settings: viewSettings }[S.view];
  el.innerHTML = tr(r());
  el.scrollTop = 0;
  const top = $('#to-top');
  if (top) top.classList.remove('show');
  if (S.view === 'home') {
    drawHero();
    loadNews();
  }
  if (S.view === 'console') scrollConsole();
  if (S.view === 'settings') bindSettings();
}

function viewHome() {
  const inst = currentInstance();
  const playing = inst && S.running.has(inst.id);
  const h = new Date().getHours();
  const greet = h < 6 ? 'Buenas noches' : h < 12 ? 'Buenos días' : h < 20 ? 'Buenas tardes' : 'Buenas noches';
  const a = activeAccount();
  const saludo = a && inst
    ? `<span class="chip">${greet}, ${esc(a.name)} 👋</span>`
    : '';
  const hero = inst
    ? `<div class="hero-title stack" style="gap:10px">
         <div class="row wrap">${saludo}<span class="chip">Minecraft ${esc(inst.mc_version)}</span><span class="chip">${LOADER_NAME[inst.loader] || esc(inst.loader)}${inst.loader_version ? ' ' + esc(inst.loader_version) : ''}</span>${inst.last_played ? '<span class="chip">⏳ Continuar donde lo dejaste</span>' : ''}</div>
         <h1>${esc(inst.name)}</h1>
       </div>
       <button class="btn primary play" data-act="${playing ? 'stop' : 'play'}" data-id="${esc(inst.id)}">${ic(playing ? 'stop' : 'play')}${playing ? 'Detener' : 'Jugar'}</button>`
    : `<div class="hero-title stack" style="gap:10px"><h1>Tu primera instancia</h1><span>Elige una versión y un cargador para empezar a jugar.</span></div>
       <button class="btn primary play" data-act="new-instance">${ic('plus')}Crear instancia</button>`;
  const recents = S.instances.slice(0, 6).map((i) => `
    <div class="card pick with-cover ${currentInstance() && currentInstance().id === i.id ? 'sel' : ''}" data-act="pick" data-id="${esc(i.id)}">
      ${i.icon ? `<img class="cover" src="${esc(i.icon)}" alt="">` : '<div class="inst-top a' + (i.accent % 6) + '"></div>'}
      <h3 class="ellipsis">${esc(i.name)}</h3>
      <span class="muted">${esc(i.mc_version)} · ${LOADER_NAME[i.loader] || esc(i.loader)}${i.playtime ? ' ⏱ ' + fmtPlaytime(i.playtime) : ''}</span>
    </div>`).join('');
  return `
    <div class="hero ${inst && inst.icon ? 'with-cover' : ''}">${inst && inst.icon ? `<img class="hero-cover" src="${esc(inst.icon)}" alt="">` : ''}<canvas id="hero-canvas"></canvas><div class="sparkle"></div><div class="hero-body">${hero}</div></div>
    ${S.instances.length ? `<div class="section"><div class="row between"><h2>Instancias recientes</h2><button class="btn small ghost" data-act="nav" data-view="instances">Ver todas</button></div><div class="grid">${recents}</div></div>` : ''}
    ${S.instances.length ? heroStats() : ''}
    <div class="section"><div class="row between"><h2>Novedades de Minecraft</h2></div><div id="news" class="news"><div class="skeleton sk-card"></div><div class="skeleton sk-card"></div><div class="skeleton sk-card"></div><div class="skeleton sk-card"></div></div></div>`;
}

function heroStats() {
  const fav = S.instances.reduce((x, y) => ((y.playtime || 0) > (x ? x.playtime : -1) ? y : x), null);
  const totalSecs = S.instances.reduce((n, i) => n + (i.playtime || 0), 0);
  return `<div class="panel row wrap" style="margin-top:16px;gap:22px">
    <span>🏆 <b>${fmtPlaytime(totalSecs)}</b> <span class="muted">jugadas en total</span></span>
    ${fav ? `<span>⭐ <b>${esc(fav.name)}</b> <span class="muted">es tu favorita</span></span>` : ''}
    <span>📦 <b>${S.instances.length}</b> <span class="muted">${S.instances.length === 1 ? 'instancia' : 'instancias'}</span></span>
  </div>`;
}

function drawHero() {
  const c = $('#hero-canvas');
  if (!c) return;
  const w = (c.width = c.clientWidth * devicePixelRatio);
  const h = (c.height = c.clientHeight * devicePixelRatio);
  const g = c.getContext('2d');
  const sky = g.createLinearGradient(0, 0, 0, h);
  sky.addColorStop(0, '#0a1020');
  sky.addColorStop(0.55, '#2a2561');
  sky.addColorStop(1, '#6a4c93');
  g.fillStyle = sky;
  g.fillRect(0, 0, w, h);
  let seed = 1337;
  const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const px = 12 * devicePixelRatio;
  for (let i = 0; i < 90; i++) {
    g.fillStyle = `rgba(255,255,255,${0.25 + rnd() * 0.6})`;
    const s = rnd() > 0.85 ? px / 2 : px / 4;
    g.fillRect(Math.floor((rnd() * w) / px) * px, Math.floor((rnd() * h * 0.55) / px) * px, s, s);
  }
  const mx = Math.floor((w * 0.78) / px) * px;
  const my = Math.floor((h * 0.2) / px) * px;
  g.fillStyle = '#ffc857';
  g.fillRect(mx, my, px * 4, px * 4);
  g.fillStyle = 'rgba(255,200,87,.18)';
  g.fillRect(mx - px, my - px, px * 6, px * 6);
  const layers = [
    { base: 0.5, amp: 5, color: '#3b3a7a' },
    { base: 0.62, amp: 6, color: '#2b3a73' },
    { base: 0.74, amp: 4, color: '#1d2d5c' },
    { base: 0.86, amp: 3, color: '#121e42' }
  ];
  const cols = Math.ceil(w / px);
  layers.forEach((L, li) => {
    let hgt = L.base * (h / px);
    for (let x = 0; x < cols; x++) {
      hgt += (rnd() - 0.5) * L.amp * 0.55;
      hgt = Math.max(L.base * (h / px) - L.amp * 3, Math.min(L.base * (h / px) + L.amp * 3, hgt));
      const top = Math.round(hgt) * px;
      g.fillStyle = L.color;
      g.fillRect(x * px, top, px, h - top);
      g.fillStyle = li === 3 ? '#5ad17d' : 'rgba(255,255,255,.06)';
      g.fillRect(x * px, top, px, li === 3 ? px / 2 : px / 3);
    }
  });
  const grad = g.createLinearGradient(0, h * 0.35, 0, h);
  grad.addColorStop(0, 'rgba(12,20,36,0)');
  grad.addColorStop(1, 'rgba(12,20,36,.88)');
  g.fillStyle = grad;
  g.fillRect(0, h * 0.35, w, h * 0.65);
}

async function loadNews() {
  const box = $('#news');
  if (!box) return;
  if (!S.news) {
    try {
      S.news = await api('patch_notes', {}, true);
    } catch (_) {
      S.news = [];
    }
  }
  const el = $('#news');
  if (!el) return;
  el.innerHTML = S.news.length
    ? S.news.map((n) => `<div class="card">${n.image ? `<img src="${esc(n.image)}" alt="" onerror="this.remove()">` : ''}<div><span class="chip" style="align-self:flex-start">${esc(n.version || '')}</span><h3>${esc(n.title)}</h3><p class="clamp">${esc(n.summary)}</p></div></div>`).join('')
    : '<div class="muted">No se pudieron cargar las novedades. Comprueba tu conexión.</div>';
}

function fmtPlaytime(sec) {
  const m = Math.floor((sec || 0) / 60);
  if (m < 1) return '—';
  const h = Math.floor(m / 60);
  return h > 0 ? `${h} h ${m % 60} min` : `${m} min`;
}

function instanceCard(i) {
  const playing = S.running.has(i.id);
  return `<div class="card with-cover ${playing ? 'playing' : ''}" draggable="true" data-id="${esc(i.id)}">
    ${i.icon ? `<img class="cover" src="${esc(i.icon)}" alt="">` : `<div class="inst-top a${i.accent % 6}"></div>`}
    <div><h3 class="ellipsis">${esc(i.name)}</h3><span class="muted">${esc(i.mc_version)} · ${LOADER_NAME[i.loader] || esc(i.loader)}${i.loader_version ? ' ' + esc(i.loader_version.replace(i.mc_version + '-', '')) : ''}</span>${playing ? '<span class="chip" style="margin-left:6px">▶ EN JUEGO</span>' : ''}<br><span class="muted" style="font-size:12.5px">Tiempo jugado: ${fmtPlaytime(i.playtime)}</span></div>
    <div class="row wrap">
      <button class="btn small primary" data-act="${playing ? 'stop' : 'play'}" data-id="${esc(i.id)}">${ic(playing ? 'stop' : 'play')}${playing ? 'Detener' : 'Jugar'}</button>
      <button class="btn small" data-act="content" data-id="${esc(i.id)}">${ic('list')}Contenido</button>
      <button class="icon-btn" title="Verificar y reparar" aria-label="Verificar y reparar" data-act="verify" data-id="${esc(i.id)}">${ic('refresh')}</button>
      <button class="icon-btn" title="Abrir carpeta" aria-label="Abrir carpeta" data-act="folder" data-id="${esc(i.id)}">${ic('folder')}</button>
      <button class="icon-btn" title="Editar" aria-label="Editar" data-act="edit" data-id="${esc(i.id)}">${ic('edit')}</button>
      <button class="icon-btn" title="Duplicar" aria-label="Duplicar" data-act="duplicate" data-id="${esc(i.id)}">${ic('copy')}</button>
      <button class="icon-btn" title="Exportar modpack" aria-label="Exportar modpack" data-act="export" data-id="${esc(i.id)}">${ic('up')}</button>
      <button class="icon-btn" title="Eliminar" aria-label="Eliminar" data-act="delete" data-id="${esc(i.id)}">${ic('trash')}</button>
    </div>
  </div>`;
}

function viewInstances() {
  const groups = [...new Set(S.instances.map((i) => i.group || ''))].sort((a, b) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b)));
  const body = groups.map((g) => {
    const items = S.instances.filter((i) => (i.group || '') === g);
    return `${groups.length > 1 ? `<h2 style="margin:22px 0 12px">${esc(g || 'Sin grupo')}</h2>` : ''}<div class="grid">${items.map(instanceCard).join('')}</div>`;
  }).join('');
  const fav = S.instances.reduce((a, b) => ((b.playtime || 0) > (a ? a.playtime : -1) ? b : a), null);
  const totalSecs = S.instances.reduce((n, i) => n + (i.playtime || 0), 0);
  const stats = S.instances.length
    ? `<div class="panel row wrap" style="margin-bottom:16px;gap:18px">
        <span>${ic('play')} <b>${S.instances.length}</b> <span class="muted">${S.instances.length === 1 ? 'instancia' : 'instancias'}</span></span>
        <span>⏱ <b>${fmtPlaytime(totalSecs)}</b> <span class="muted">en total</span></span>
        ${fav && (fav.playtime || 0) > 0 ? `<span>⭐ <b>${esc(fav.name)}</b> <span class="muted">${fmtPlaytime(fav.playtime)}</span></span>` : ''}
      </div>`
    : '';
  return `<div class="row between wrap" style="margin-bottom:20px"><h1>Instancias</h1><div class="row"><button class="btn" data-act="import-instance">${ic('down')}Importar instancia</button><button class="btn" data-act="import-pack">${ic('down')}Importar modpack</button><button class="btn primary" data-act="new-instance">${ic('plus')}Nueva instancia</button></div></div>
    ${stats}
    ${S.instances.length ? body : `<div class="empty"><h3>Aún no tienes instancias</h3><p>Cada instancia tiene sus propios mods, mundos y ajustes.</p><button class="btn primary" data-act="new-instance">${ic('plus')}Crear instancia</button></div>`}`;
}

function viewDiscover() {
  const d = S.discover;
  const tabs = KINDS.map(([k, l]) => `<button class="tab ${d.kind === k ? 'on' : ''}" data-act="dkind" data-kind="${k}">${l}</button>`).join('');
  const needsTarget = d.kind !== 'modpack';
  const targetSel = needsTarget
    ? `<select id="d-target" aria-label="Instancia de destino">${S.instances.map((i) => `<option value="${esc(i.id)}" ${d.target === i.id ? 'selected' : ''}>${esc(i.name)} (${esc(i.mc_version)})</option>`).join('') || '<option value="">Sin instancias</option>'}</select>`
    : '';
  return `<h1 style="margin:6px 0 0">Descubrir</h1>
    <div class="tabs">${tabs}</div>
    <div class="toolbar">
      <input type="search" id="d-query" placeholder="Buscar en ${KINDS.find((k) => k[0] === d.kind)[1].toLowerCase()}" value="${esc(d.query)}">
      <select id="d-sort" aria-label="Ordenar"><option value="relevance">Relevancia</option><option value="downloads">Descargas</option><option value="newest">Más nuevos</option><option value="updated">Actualizados</option></select>
      ${targetSel}
    </div>
    <div id="d-results" class="section" style="margin-top:20px">${discoverResults()}</div>`;
}

function discoverResults() {
  const d = S.discover;
  if (d.error) return `<div class="empty"><p>${esc(d.error)}</p></div>`;
  if (!d.items.length) return d.loading ? `<div class="news" style="grid-auto-flow:row">${'<div class="card mod skeleton sk-big"></div>'.repeat(4)}</div>` : '<div class="empty"><p>Sin resultados. Prueba con otra búsqueda.</p></div>';
  const cards = d.items.map((it) => `
    <div class="card mod">
      ${it.icon ? `<img src="${esc(it.icon)}" alt="" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement('div'),{className:'ph'}))">` : '<div class="ph"></div>'}
      <div class="stack" style="gap:6px;min-width:0">
        <div><h3 class="ellipsis">${esc(it.title)}</h3><span class="muted" style="font-size:12.5px">${esc(it.author)} · ${fmtNum(it.downloads)} descargas</span></div>
        <p class="clamp">${esc(it.description)}</p>
        <div class="row"><button class="btn small primary" data-act="install" data-id="${esc(it.id)}" data-title="${esc(it.title)}">${ic('down')}${d.kind === 'modpack' ? 'Instalar como instancia' : 'Instalar'}</button><button class="btn small" data-act="details" data-id="${esc(it.id)}">Detalles</button></div>
      </div>
    </div>`).join('');
  return `<div class="grid" style="grid-template-columns:repeat(auto-fill,minmax(340px,1fr))">${cards}</div>
    ${d.items.length < d.total || d.kind === 'map' ? `<div class="row" style="justify-content:center;margin-top:20px"><button class="btn" data-act="dmore" ${d.loading ? 'disabled' : ''}>${d.loading ? 'Cargando…' : 'Cargar más'}</button></div>` : ''}`;
}

let searchTimer = null;
async function runSearch(reset) {
  const d = S.discover;
  if (reset) {
    d.offset = 0;
    d.items = [];
    d.total = 0;
  }
  d.error = '';
  d.loading = true;
  refreshResults();
  const inst = S.instances.find((i) => i.id === d.target);
  try {
    const res = await api('search', {
      kind: d.kind,
      query: d.query,
      mcVersion: inst && d.kind !== 'modpack' ? inst.mc_version : null,
      loader: inst && d.kind === 'mod' ? inst.loader : null,
      sort: d.sort,
      offset: d.offset
    }, true);
    d.items = d.items.concat(res.items);
    d.total = res.total;
    d.offset = d.items.length;
  } catch (e) {
    d.error = typeof e === 'string' ? e : 'No se pudo completar la búsqueda';
  }
  d.loading = false;
  refreshResults();
}

function refreshResults() {
  const el = $('#d-results');
  if (el && S.view === 'discover') el.innerHTML = discoverResults();
}

function viewAccounts() {
  const rows = S.accounts.map((a) => `
    <div class="item">
      <img class="skin" src="${headUrl(a)}" alt="">
      <div class="grow"><h3>${esc(a.name)}</h3><span class="muted">${a.kind === 'microsoft' ? 'Cuenta Microsoft' : 'Sin conexión'}</span></div>
      ${a.kind === 'microsoft' ? `<button class="btn small" data-act="wardrobe" data-id="${esc(a.id)}">Skins y capas</button>` : `<button class="btn small" data-act="wardrobe" data-id="${esc(a.id)}">Skin local</button>`}
      ${a.id === S.selectedAccount ? '<span class="chip">En uso</span>' : `<button class="btn small" data-act="select-account" data-id="${esc(a.id)}">Usar</button>`}
      <button class="icon-btn" aria-label="Quitar cuenta" data-act="remove-account" data-id="${esc(a.id)}">${ic('trash')}</button>
    </div>`).join('');
  return `<h1 style="margin:6px 0 20px">Cuentas</h1>
    <div class="two">
      <div class="panel stack">
        <h2>Microsoft</h2>
        <p class="muted">Inicia sesión con tu cuenta que posee Minecraft Java para jugar en servidores oficiales.</p>
        <div><button class="btn primary" data-act="ms-login">Iniciar sesión con Microsoft</button></div>
      </div>
      <div class="panel stack">
        <h2>Sin conexión</h2>
        <p class="muted">Juega en modo individual o en servidores sin verificación con cualquier nombre.</p>
        <div class="row"><input type="text" id="offline-name" maxlength="16" placeholder="Nombre de jugador"><button class="btn" data-act="add-offline">Añadir</button></div>
      </div>
    </div>
    <div class="section"><h2 style="margin-bottom:14px">Tus cuentas</h2>${rows ? `<div class="list">${rows}</div>` : '<div class="empty"><p>Todavía no hay cuentas añadidas.</p></div>'}</div>`;
}

function viewConsole() {
  return `<div class="row between" style="margin:6px 0 16px"><h1>Consola</h1><button class="btn small" data-act="clear-logs">Limpiar</button></div>
    <div class="console" id="console">${S.logs.map(logHtml).join('')}</div>`;
}

function logHtml(l) {
  const cls = /\b(ERROR|FATAL|Exception)\b/.test(l) ? 'e' : /\bWARN\b/.test(l) ? 'w' : '';
  return `<div class="${cls}">${esc(l)}</div>`;
}

function scrollConsole() {
  const c = $('#console');
  if (c) c.scrollTop = c.scrollHeight;
}

function viewSettings() {
  const s = S.settings;
  const total = S.sys ? S.sys.total_memory_mb : 8192;
  return `<h1 style="margin:6px 0 20px">Ajustes</h1>
    <div class="stack" style="max-width:820px">
      <div class="panel stack">
        <h2>Memoria</h2>
        <div class="field"><span>Máxima: <b class="mem-val" id="s-max-val">${s.max_memory} MB</b> de ${total} MB</span><input type="range" id="s-max" min="1024" max="${total}" step="256" value="${s.max_memory}"></div>
        <div class="field"><span>Mínima: <b class="mem-val" id="s-min-val">${s.min_memory} MB</b></span><input type="range" id="s-min" min="256" max="${total}" step="256" value="${s.min_memory}"></div>
      </div>
      <div class="panel stack">
        <h2>Java y juego</h2>
        <label class="field"><span>Ruta de Java (vacío = automática, se descarga si hace falta)</span><input type="text" id="s-java" value="${esc(s.java_path)}" placeholder="Ejemplo: C:\\Program Files\\Java\\bin\\java.exe (Windows) o /usr/lib/jvm (Linux)"></label>
        <label class="field"><span>Argumentos de JVM</span><textarea id="s-jvm" placeholder="-XX:+UseG1GC">${esc(s.jvm_args)}</textarea></label>
        <div class="row wrap">${Object.keys(JVM_PRESETS).map((p) => `<button class="btn small" data-act="jvm-preset" data-target="s-jvm" data-preset="${esc(p)}">${esc(p)}</button>`).join('')}</div>
        <label class="field"><span>Comando envoltorio (por ejemplo gamemoderun o mangohud)</span><input type="text" id="s-wrap" value="${esc(s.wrapper)}"></label>
        <div class="two">
          <label class="field"><span>Ancho de ventana</span><input type="number" id="s-w" min="320" value="${s.width}"></label>
          <label class="field"><span>Alto de ventana</span><input type="number" id="s-h" min="240" value="${s.height}"></label>
        </div>
        <label class="field"><span>Descargas simultáneas</span><input type="number" id="s-conc" min="1" max="128" value="${s.concurrency}"></label>
        <label class="field"><span>Copia de mundos automática mientras juegas (minutos, 0 = apagada)</span><input type="number" id="s-backup" min="0" max="240" value="${s.auto_backup_minutes || 0}"></label>
      </div>
      <div class="panel stack">
        <h2>Sistema</h2>
        <label class="field"><span>Variables de entorno (una por línea, CLAVE=valor)</span><textarea id="s-env" placeholder="MESA_GLTHREAD=true">${esc(s.env_vars)}</textarea></label>
        <label class="field"><span>Comando antes de jugar</span><input type="text" id="s-pre" value="${esc(s.pre_launch)}"></label>
        <label class="field"><span>Comando al cerrar el juego</span><input type="text" id="s-post" value="${esc(s.post_exit)}"></label>
      </div>
      <div class="panel stack">
        <h2>Aplicación</h2>
        <label class="field"><span>Idioma</span><select id="s-lang"><option value="es" ${s.language === 'es' ? 'selected' : ''}>Español</option><option value="en" ${s.language === 'en' ? 'selected' : ''}>Inglés</option></select></label>
        <label class="field"><span>Interfaz</span><select id="s-theme"><option value="dark" ${viewTheme() === 'dark' ? 'selected' : ''}>Aurora oscuro</option><option value="light" ${viewTheme() === 'light' ? 'selected' : ''}>Aurora claro</option><option value="contrast" ${viewTheme() === 'contrast' ? 'selected' : ''}>Blanco y negro (elegante)</option></select></label>
        <label class="row"><input type="checkbox" id="s-sound" ${localStorage.getItem('nebula-sound') !== '0' ? 'checked' : ''}> <span class="muted">Sonidos del launcher (inicio y fin de partida)</span></label>
        <label class="field"><span>Repositorio de actualizaciones (usuario/repo en GitHub)</span><input type="text" id="s-repo" value="${esc(s.update_repo)}" placeholder="usuario/nebula-launcher"></label>
        <label class="row"><input type="checkbox" id="s-tray" ${s.close_to_tray !== false ? 'checked' : ''}> <span class="muted">Minimizar a la bandeja del sistema al cerrar la ventana</span></label>
      </div>
      <div class="panel stack">
        <h2>Discord Rich Presence</h2>
        <label class="field"><span>Discord Application ID (vacío = desactivado)</span><input type="text" id="s-discord" value="${esc(s.discord_app_id || '')}" placeholder="1234567890123456789"></label>
        <p class="muted">Crea una aplicación en discord.com/developers/applications y pega aquí su Application ID para mostrar "Jugando a…" en tu perfil mientras juegas.</p>
      </div>
      <div class="panel stack">
        <h2>Cuentas y servicios</h2>
        <p class="muted">El inicio de sesión con Microsoft se hace ahora a través de un servidor local (localhost), por lo que no hace falta configurar ningún Client ID de Azure.</p>
        <p class="muted">Curse Forge ha sido desactivado en este lanzador.</p>
      </div>
      <div class="panel stack">
        <h2>Acerca de</h2>
        <p class="muted">Nebula Launcher — lanzador de Minecraft Java de código abierto (MIT).</p>
        <div class="row wrap"><button class="btn small" data-act="about">Acerca de Nebula</button></div>
      </div>
      <div class="row"><button class="btn primary" data-act="save-settings">Guardar cambios</button><button class="btn" data-act="folder-root">${ic('folder')}Abrir carpeta de datos</button></div>
    </div>`;
}

function bindSettings() {
  const max = $('#s-max');
  const min = $('#s-min');
  max.oninput = () => ($('#s-max-val').textContent = max.value + ' MB');
  min.oninput = () => ($('#s-min-val').textContent = min.value + ' MB');
  const snd = $('#s-sound');
  if (snd) snd.onchange = () => localStorage.setItem('nebula-sound', snd.checked ? '1' : '0');
  const thm = $('#s-theme');
  if (thm) thm.onchange = () => {
    S.settings.theme = thm.value;
    applyTheme();
    document.body.classList.toggle('theme-preview-light', thm.value === 'light');
    document.body.classList.toggle('theme-preview-contrast', thm.value === 'contrast');
  };
}

const scrollable = () => document.getElementById('view');
document.addEventListener('keydown', (e) => {
  const t = e.target;
  if (t && (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || t.isContentEditable)) return;
  const v = scrollable();
  if (!v) return;
  const step = 70;
  if (e.key === 'ArrowDown' || e.key === 'PageDown' || (e.key === ' ' && !e.shiftKey)) {
    e.preventDefault();
    v.scrollBy({ top: e.key !== 'ArrowDown' ? 320 : step, behavior: 'smooth' });
  } else if (e.key === 'ArrowUp' || e.key === 'PageUp' || (e.key === ' ' && e.shiftKey)) {
    e.preventDefault();
    v.scrollBy({ top: e.key !== 'ArrowUp' ? -320 : -step, behavior: 'smooth' });
  } else if (e.key === 'Home') {
    e.preventDefault();
    v.scrollTo({ top: 0, behavior: 'smooth' });
  } else if (e.key === 'End') {
    e.preventDefault();
    v.scrollTo({ top: v.scrollHeight, behavior: 'smooth' });
  }
});

let panScroll = null;
document.addEventListener('mousedown', (e) => {
  if (e.button !== 1) return;
  const v = scrollable();
  if (!v || !v.contains(e.target)) return;
  e.preventDefault();
  panScroll = { y: e.clientY, top: v.scrollTop, view: v };
  v.classList.add('panning');
});
document.addEventListener('mousemove', (e) => {
  if (!panScroll) return;
  panScroll.view.scrollTop = panScroll.top - (e.clientY - panScroll.y);
});
const endPan = () => {
  if (panScroll) {
    panScroll.view.classList.remove('panning');
    panScroll = null;
  }
};
document.addEventListener('mouseup', endPan);
document.addEventListener('auxclick', (e) => { if (e.button === 1) e.preventDefault(); });

function showModal(html, wide = false) {
  const m = $('#modal');
  m.innerHTML = tr(`<div class="sheet ${wide ? 'wide' : ''}" role="dialog" aria-modal="true">${html}</div>`);
  m.hidden = false;
  const first = m.querySelector('input, select, button.primary');
  if (first) first.focus();
}

function closeModal() {
  S.content.open = false;
  $('#modal').hidden = true;
  $('#modal').innerHTML = '';
}

function sheetHead(title) {
  return `<header><h2>${title}</h2><button class="icon-btn" data-act="close" aria-label="Cerrar">${ic('x')}</button></header>`;
}

async function ensureVersions() {
  if (!S.versions) S.versions = await api('versions');
}

async function openCreate() {
  try {
    await ensureVersions();
  } catch (_) {
    return;
  }
  showModal(`${sheetHead('Nueva instancia')}
    <div class="stack">
      <label class="field"><span>Nombre</span><input type="text" id="c-name" maxlength="40" placeholder="Supervivencia con amigos"></label>
      <div class="field"><span>Cargador</span>
        <div class="seg" id="c-loader">${LOADERS.map(([v, l], n) => `<button data-act="cloader" data-v="${v}" class="${n === 0 ? 'on' : ''}">${l}</button>`).join('')}</div>
      </div>
      <div class="two">
        <label class="field"><span>Versión de Minecraft</span><select id="c-version"></select></label>
        <label class="field"><span>Versión del cargador</span><select id="c-lv" disabled><option value="">No aplica</option></select></label>
      </div>
      <label class="row"><input type="checkbox" id="c-snap" ${S.settings.show_snapshots ? 'checked' : ''}> <span class="muted">Mostrar snapshots y versiones antiguas</span></label>
    </div>
    <footer><button class="btn" data-act="close">Cancelar</button><button class="btn primary" data-act="create-submit" id="c-submit">Crear</button></footer>`);
  fillVersions();
  $('#c-snap').onchange = fillVersions;
  $('#c-version').onchange = loadLoaderVersions;
}

function fillVersions() {
  const snaps = $('#c-snap').checked;
  const list = S.versions.versions.filter((v) => snaps || v.kind === 'release');
  $('#c-version').innerHTML = list.map((v) => `<option value="${esc(v.id)}" ${v.id === S.versions.latest_release ? 'selected' : ''}>${esc(v.id)}${v.id === S.versions.latest_release ? ' (última)' : ''}</option>`).join('');
  loadLoaderVersions();
}

async function loadLoaderVersions() {
  const loader = $('#c-loader .on').dataset.v;
  const sel = $('#c-lv');
  if (loader === 'vanilla') {
    sel.disabled = true;
    sel.innerHTML = '<option value="">No aplica</option>';
    $('#c-submit').disabled = false;
    return;
  }
  sel.disabled = true;
  sel.innerHTML = '<option>Cargando…</option>';
  const mc = $('#c-version').value;
  try {
    const list = await api('loader_versions', { kind: loader, mcVersion: mc }, true);
    if ($('#c-loader .on').dataset.v !== loader || $('#c-version').value !== mc) return;
    if (!list.length) {
      sel.innerHTML = '<option value="">No disponible para esta versión</option>';
      $('#c-submit').disabled = true;
      return;
    }
    sel.innerHTML = list.map((v, i) => `<option value="${esc(v)}">${esc(v)}${i === 0 ? ' (última)' : ''}</option>`).join('');
    sel.disabled = false;
    $('#c-submit').disabled = false;
  } catch (_) {
    sel.innerHTML = '<option value="">Error de red</option>';
    $('#c-submit').disabled = true;
  }
}

function openEdit(id) {
  const i = S.instances.find((x) => x.id === id);
  if (!i) return;
  S.editAccent = i.accent % 6;
  showModal(`${sheetHead('Editar instancia')}
    <div class="stack">
      <div class="row" style="align-items:center;gap:14px">
        ${i.icon ? `<img class="icon-preview" src="${esc(i.icon)}" alt="">` : '<div class="icon-preview"></div>'}
        <div class="row" id="e-icon-btns" data-id="${esc(id)}"><button class="btn small" data-act="icon-pick" data-id="${esc(id)}">Cambiar icono (PNG)</button><button class="btn small" data-act="icon-remove" data-id="${esc(id)}" ${i.icon ? '' : 'disabled'}>Quitar icono</button></div>
      </div>
      <span class="chip" id="e-java-chip">⏳ Comprobando versión de Java…</span>
      <label class="field"><span>Nombre</span><input type="text" id="e-name" maxlength="40" value="${esc(i.name)}"></label>
      <div class="two">
        <label class="field"><span>Grupo</span><input type="text" id="e-group" maxlength="40" value="${esc(i.group || '')}" placeholder="Supervivencia"></label>
        <div class="field"><span>Color</span><div class="row" id="e-accent">${[0, 1, 2, 3, 4, 5].map((n) => `<button class="swatch a${n} ${n === S.editAccent ? 'on' : ''}" data-act="accent" data-n="${n}" aria-label="Color ${n + 1}"></button>`).join('')}</div></div>
      </div>
      <label class="field"><span>Memoria máxima en MB (vacío = global)</span><input type="number" id="e-mem" min="512" step="256" value="${i.memory_mb || ''}"></label>
      <label class="field"><span>Ruta de Java (vacío = global)</span><input type="text" id="e-java" value="${esc(i.java_path || '')}"></label>
      <label class="field"><span>Argumentos de JVM adicionales</span><textarea id="e-jvm">${esc(i.jvm_args || '')}</textarea></label>
      <div class="row wrap">${Object.keys(JVM_PRESETS).map((p) => `<button class="btn small" data-act="jvm-preset" data-target="e-jvm" data-preset="${esc(p)}">${esc(p)}</button>`).join('')}</div>
      <label class="field"><span>Variables de entorno de la instancia</span><textarea id="e-env" placeholder="MESA_GLTHREAD=true">${esc(i.env_vars || '')}</textarea></label>
    </div>
    <footer><button class="btn" data-act="close">Cancelar</button><button class="btn primary" data-act="edit-submit" data-id="${esc(id)}">Guardar</button></footer>`);
  api('java_needed', { id }, true).then((mj) => {
    const chip = $('#e-java-chip');
    if (chip && mj) chip.textContent = `☕ Requiere Java ${mj} (se descarga automáticamente si falta)`;
  }).catch(() => {
    const chip = $('#e-java-chip');
    if (chip) chip.remove();
  });
}

async function openContent(id, tab) {
  S.content.id = id;
  S.content.open = true;
  if (tab) {
    if (tab !== S.content.tab) S.content.updates = null;
    S.content.tab = tab;
  }
  const t = S.content.tab;
  const inst = S.instances.find((i) => i.id === id);
  let items = [];
  if (t !== 'servers' && t !== 'screens') {
    try {
      items = await api('list_content', { id, kind: t });
    } catch (_) {
      return;
    }
  }
  S.content.items = items;
  const tabs = CONTENT_TABS.map(([k, l]) => `<button class="tab ${t === k ? 'on' : ''}" data-act="ctab" data-tab="${k}">${l}</button>`).join('');
  let toolbar = '';
  if (t === 'mods') toolbar = `<button class="btn small primary" data-act="optimize" data-id="${esc(id)}">⚡Optimizar</button><button class="btn small" data-act="add-files">${ic('plus')}Añadir archivo</button><button class="btn small" data-act="check-updates">${ic('refresh')}Buscar actualizaciones</button>${inst && inst.pack_project ? `<button class="btn small" data-act="mp-refresh" data-id="${esc(id)}">${ic('refresh')}Actualizar modpack</button>` : ''}`;
  else if (t === 'resourcepacks' || t === 'shaderpacks') toolbar = `<button class="btn small" data-act="add-files">${ic('plus')}Añadir archivo</button>`;
  else if (t === 'saves' || t === 'backups') toolbar = `<button class="btn small" data-act="backup">${ic('copy')}Crear copia ahora</button>`;
  const ups = S.content.updates;
  const banner = t === 'mods' && ups
    ? ups.length
      ? `<div class="panel stack" style="margin-top:14px"><div class="row between"><b>${ups.length} actualizaciones disponibles</b><button class="btn small primary" data-act="apply-all">Actualizar todo</button></div>${ups.map((u) => `<div class="row between"><span class="ellipsis">${esc(u.file)} → ${esc(u.new_version)}</span><button class="btn small" data-act="apply-update" data-name="${esc(u.file)}" data-vid="${esc(u.version_id)}">Actualizar</button></div>`).join('')}</div>`
      : '<div class="muted" style="margin-top:14px">Todo está al día</div>'
    : '';
  let body = '';
  if (t === 'servers') {
    const servers = (inst && inst.servers) || [];
    const addr = (sv) => String(sv.address).replace(/"/g, '&quot;');
    body = `${servers.length ? servers.map((sv, n) => `<div class="item"><span class="srv-dot" id="srvdot-${n}" title=""></span><div class="grow"><div class="ellipsis">${esc(sv.name)}<span class="muted srv-info" id="srvinfo-${n}" style="font-size:12px;margin-left:8px"></span></div><span class="muted ellipsis" style="font-size:12.5px;display:block">${esc(sv.address)}</span></div><button class="btn small primary" data-act="play-server" data-address="${addr(sv)}">${ic('play')}Entrar</button><button class="icon-btn" aria-label="Eliminar" data-act="sv-del" data-n="${n}">${ic('trash')}</button></div>`).join('') : '<div class="empty"><p>Aún no hay servidores. Añade uno para entrar directo al iniciar.</p></div>'}
      <div class="row wrap" style="margin-top:6px"><input type="text" id="sv-name" placeholder="Nombre del servidor" style="flex:1;min-width:140px"><input type="text" id="sv-addr" placeholder="Dirección del servidor" style="flex:2;min-width:180px"><button class="btn" data-act="sv-add">${ic('plus')}Añadir</button><button class="btn small" data-act="sv-refresh">${ic('refresh')}Actualizar estado</button><button class="btn small" data-act="srv-popular">⭐ Servidores populares</button></div>`;
    setTimeout(() => fetchServerStatuses(servers), 30);
  } else if (t === 'screens') {
    body = items.length ? `<div class="grid" style="grid-template-columns:repeat(auto-fill,minmax(180px,1fr))">${items.map((it) => `<div class="card mod" style="padding:8px"><img src="${esc(it.thumb)}" alt="" style="width:100%;border-radius:8px" loading="lazy"><div class="stack" style="gap:6px"><span class="muted" style="font-size:12px">${new Date((it.when || 0) * 1000).toLocaleString()}</span><button class="btn small" data-act="shot-icon" data-id="${esc(id)}" data-name="${esc(it.name)}">Usar como icono</button></div></div>`).join('')}</div>` : '<div class="empty"><p>No hay capturas todavía. Las capturas aparecen aquí cuando las haces dentro del juego (F2).</p></div>';
  } else if (t === 'backups') {
    body = (items.length ? items.map((it) => `<div class="item"><div class="grow"><div class="ellipsis">${esc(it.name)}</div><span class="muted" style="font-size:12.5px">${fmtSize(it.size)}</span></div><button class="btn small" data-act="restore" data-name="${esc(it.name)}">Restaurar</button><button class="icon-btn" aria-label="Eliminar" data-act="cremove" data-name="${esc(it.name)}">${ic('trash')}</button></div>`).join('') : '<div class="empty"><p>Las copias guardan todos los mundos de la instancia.</p></div>');
  } else {
    body = items.length ? items.map((it) => {
      const title = it.title || it.name.replace(/\.disabled$/, '');
      const sub = t === 'mods' && it.title ? `${it.version ? esc(it.version) + ' · ' : ''}${esc(it.name.replace(/\.disabled$/, ''))}` : it.is_dir ? 'Carpeta' : fmtSize(it.size);
      return `<div class="item ${it.enabled ? '' : 'off'}" data-modfile="${esc(it.name)}">
        ${t === 'mods' ? `<span class="ms-dot" data-file="${esc(it.name)}"></span>${it.icon ? `<img class="skin" src="${esc(it.icon)}" alt="">` : '<div class="skin" style="background:var(--raised)"></div>'}` : ''}
        <div class="grow"><div class="ellipsis">${esc(title)}</div><span class="muted ellipsis" style="font-size:12.5px;display:block">${sub}${it.enabled ? '' : ' · desactivado'}</span></div>
        ${t !== 'saves' ? `<button class="btn small" data-act="ctoggle" data-name="${esc(it.name)}">${it.enabled ? 'Desactivar' : 'Activar'}</button>` : ''}
        <button class="icon-btn" aria-label="Eliminar" data-act="cremove" data-name="${esc(it.name)}">${ic('trash')}</button>
      </div>`;
    }).join('') : '<div class="empty"><p>No hay nada aquí todavía. Búscalo en Descubrir.</p></div>';
  }
  const hint = ['mods', 'resourcepacks', 'shaderpacks'].includes(t) ? '<p class="muted" style="margin-top:14px;font-size:13px">Arrastra archivos aquí para añadirlos</p>' : '';
  showModal(`${sheetHead(esc(inst ? inst.name : 'Contenido'))}
    <div class="row between wrap"><div class="tabs" style="margin:0">${tabs}</div><div class="row wrap">${toolbar}<button class="btn small" data-act="folder" data-id="${esc(id)}" data-sub="${t === 'servers' || t === 'backups' ? '' : t === 'screens' ? 'screenshots' : t}">${ic('folder')}Abrir carpeta</button></div></div>
    ${banner}<div class="list" style="margin-top:14px">${body}</div>${hint}`, true);
  if (t === 'mods') {
    api('check_mod_status', { id }, true).then((st) => {
      if (!st || S.content.id !== id) return;
      const byFile = new Map(st.items.map((i) => [i.file, i]));
      document.querySelectorAll('#modal .ms-dot[data-file]').forEach((el) => {
        const it = byFile.get(el.dataset.file);
        if (!it) return;
        el.className = `ms-dot ${it.state}`;
        el.title = it.state === 'ok' ? 'Al día en Modrinth' : it.state === 'update' ? `Actualización disponible → ${it.new_version}` : 'No está en Modrinth (instalado a mano)';
        if (it.state === 'update') el.innerHTML = '⬆';
      });
      const legend = document.createElement('p');
      legend.className = 'muted';
      legend.style.cssText = 'margin-top:10px;font-size:12.5px';
      legend.innerHTML = `🔴🟡🟢 Semáforo: <b style="color:#4ade80">${st.ok} al día</b> · <b style="color:#facc15">${st.updated} con actualización</b> · <b>${st.unknown} ajenos a Modrinth</b>`;
      const list = document.querySelector('.list');
      if (list) list.appendChild(legend);
    }).catch(() => {});
  }
}

async function fetchServerStatuses(servers) {
  servers.forEach((sv, n) => {
    const dot = document.getElementById(`srvdot-${n}`);
    const info = document.getElementById(`srvinfo-${n}`);
    if (dot) { dot.className = 'srv-dot wait'; dot.title = tr('Consultando…'); dot.textContent = '…'; }
    if (info) info.textContent = '';
    api('ping_server', { address: sv.address }, true).then((st) => {
      const d = document.getElementById(`srvdot-${n}`);
      const i = document.getElementById(`srvinfo-${n}`);
      if (!d) return;
      if (st && st.online) {
        d.className = 'srv-dot on';
        d.title = `${esc(st.motd)} — ${esc(st.version)} — ${st.ping_ms} ms`;
        if (i) i.textContent = `${st.players_online}/${st.players_max} · ${esc(st.version)} · ${st.ping_ms} ms`;
      } else {
        d.className = 'srv-dot off';
        d.title = tr('Sin conexión');
        if (i) i.textContent = tr('Sin conexión');
      }
    }).catch(() => {
      const d = document.getElementById(`srvdot-${n}`);
      if (d) d.className = 'srv-dot off';
    });
  });
}

function analyzeCrash(text) {
  const rules = [
    [/OutOfMemoryError|GC overhead limit/, 'Falta memoria. Sube la memoria máxima en Ajustes o en la instancia.'],
    [/UnsupportedClassVersionError|class file version|has been compiled by a more recent version/, 'La versión de Java no coincide. Deja la ruta de Java vacía para que Nebula use la correcta.'],
    [/Mixin apply failed|MixinTransformerError|InvalidMixinException|MixinApplyError/, 'Un mod no es compatible con esta versión del juego o con otro mod. Desactiva los mods recientes e inténtalo de nuevo.'],
    [/Incompatible mod set|Missing or unsupported mandatory dependencies|requires mod|Could not find required mod|ModResolutionException|depends on/i, 'Falta una dependencia. Instala la librería que pide el mod, como Fabric API.'],
    [/GLFW error|Failed to create window|EGL|OpenGL|GLX|GL_INVALID/i, 'Problema gráfico. Actualiza los drivers, desactiva los shaders o prueba otra variable de entorno OpenGL.'],
    [/Exit code 137|Killed|oom-kill/i, 'El sistema cerró el juego por falta de RAM. Baja la memoria asignada o cierra otras aplicaciones.']
  ];
  return rules.filter(([re]) => re.test(text)).map(([, msg]) => msg);
}

async function showCrash(id) {
  let info = { report: null, log_tail: '' };
  try {
    info = await api('crash_report', { id }, true);
  } catch (_) {}
  const recent = S.logs.slice(-120).join('\n');
  const hints = analyzeCrash(`${info.report || ''}\n${info.log_tail || ''}\n${recent}`);
  const shown = (info.report || info.log_tail || recent).split('\n').slice(-80).join('\n');
  S.lastCrash = shown;
  showModal(`${sheetHead('El juego se cerró con un error')}
    <div class="stack">
      <h3>Posibles causas</h3>
      ${hints.length ? hints.map((h) => `<div class="item">${esc(h)}</div>`).join('') : '<p class="muted">No se encontró un patrón conocido. Copia el registro y compártelo para pedir ayuda.</p>'}
      <div class="console" style="height:220px;min-height:0">${esc(shown)}</div>
    </div>
    <footer><button class="btn" data-act="close">Cerrar</button><button class="btn primary" data-act="copy-log">Copiar registro</button></footer>`, true);
}

let wardrobeAcc = null;

function drawCape(canvas, url) {
  const img = new Image();
  img.onerror = () => { canvas.remove(); };
  img.onload = () => {
    const s = 8;
    canvas.width = 10 * s;
    canvas.height = 16 * s;
    const c = canvas.getContext('2d');
    c.imageSmoothingEnabled = false;
    c.drawImage(img, 1, 1, 10, 16, 0, 0, 10 * s, 16 * s);
  };
  img.src = url;
}

function paintWardrobe() {
  document.querySelectorAll('#modal canvas[data-cape]').forEach((c) => drawCape(c, c.dataset.cape));
}

function skinCube(url) {
  return `<span class="skin-cube-wrap"><span class="skin-cube" style="--tex:url('${url}')"><i class="f0"></i><i class="f1"></i><i class="f2"></i><i class="f3"></i><i class="f4"></i><i class="f5"></i></span></span>`;
}

async function openPopular() {
  const id = S.content.id;
  if (!id) return;
  const cur = S.instances.find((i) => i.id === id);
  const owned = new Set(((cur && cur.servers) || []).map((s) => s.address));
  showModal(`${sheetHead('Servidores populares')}
    <p class="muted">Servidores grandes y conocidos con ping real en vivo. Haz clic en ${ic('plus')} Añadir para ponerlos en tu instancia (algunos necesitan cuenta premium de Java).</p>
    <div class="list">${POPULAR_SERVERS.map(([n, a], idx) => `
      <div class="item">
        <span class="srv-dot wait" id="psdot-${idx}" title=""></span>
        <div class="grow"><div>${esc(n)}</div><span class="muted" style="font-size:12.5px">${esc(a)} · ${esc(['Snn purpur','Minigames','Bedwars/SkyWars','Caja de supervivencia','Prision','Supervivencia anárquica','Anarquía histórica'][idx] || 'Multijugador')}</span><span class="muted srv-info" id="psinfo-${idx}" style="font-size:12px;display:block"></span></div>
        ${owned.has(a) ? '<span class="chip">AÑADIDO</span>' : `<button class="btn small primary" data-act="pop-add" data-name="${esc(n)}" data-addr="${esc(a)}">${ic('plus')}Añadir</button>`}
      </div>`).join('')}</div>
    <footer><button class="btn primary" data-act="close">Cerrar</button></footer>`, true);
  POPULAR_SERVERS.forEach(async ([, a], idx) => {
    try {
      const st = await api('ping_server', { address: a }, true);
      const dot = document.getElementById(`psdot-${idx}`);
      const info = document.getElementById(`psinfo-${idx}`);
      if (!dot) return;
      if (st && st.online) {
        dot.className = 'srv-dot on';
        if (info) info.textContent = `${st.players_online} jugadores · ${st.ping_ms} ms`;
      } else {
        dot.className = 'srv-dot off';
        if (info) info.textContent = 'Sin conexión ahora mismo';
      }
    } catch (_) {
      const dot = document.getElementById(`psdot-${idx}`);
      if (dot) dot.className = 'srv-dot off';
    }
  });
}

async function openWardrobe(id) {
  const acc = S.accounts.find((a) => a.id === id);
  if (!acc) return;
  wardrobeAcc = id;
  const isMs = acc.kind === 'microsoft';
  const [skins, profile] = await Promise.all([
    api('wardrobe_list'),
    isMs ? api('profile_items', { accountId: id }, true).catch(() => null) : Promise.resolve(null)
  ]);
  const allSkins = skins || [];
  const saved = allSkins.length
    ? allSkins.map((s) => `
      <div class="item">
        ${skinCube(s.data)}
        <div class="grow"><h3 class="ellipsis">${esc(s.name)}</h3></div>
        ${isMs ? `<button class="btn small primary" data-act="wardrobe-apply" data-id="${esc(s.id)}">Aplicar</button>` : ''}
        <button class="icon-btn" aria-label="Eliminar" data-act="wardrobe-remove" data-id="${esc(s.id)}">${ic('trash')}</button>
      </div>`).join('')
    : '<div class="empty"><p>El armario está vacío. Añade skins con una URL o desde un archivo PNG.</p></div>';
  let capesPart = '';
  if (isMs && profile) {
    capesPart = profile.capes && profile.capes.length
      ? profile.capes.map((c) => `
        <div class="item">
          <canvas class="cape-canvas" data-cape="${esc(c.url)}"></canvas>
          <div class="grow"><h3 class="ellipsis">${esc(c.alias || 'Capa')}</h3>${c.active ? '<span class="chip">ACTIVA</span>' : ''}</div>
          ${c.active ? '<button class="btn small" data-act="cape-clear">Quitar</button>' : `<button class="btn small" data-act="cape-on" data-id="${esc(c.id)}">Poner</button>`}
        </div>`).join('')
      : '<p class="muted">Tu cuenta no tiene capas todavía. Las capas se consiguen en minecraft.net con el perfil de la cuenta y ahí es donde aparecen listadas.</p>';
  } else if (!isMs) {
    const insts = S.instances.filter((i) => i.loader !== 'vanilla');
    const sel = insts.length
      ? `<label class="field" style="max-width:260px"><span>Instancia</span><select id="w-off-inst">${insts.map((i) => `<option value="${esc(i.id)}">${esc(i.name)} (${esc(i.mc_version)})</option>`).join('')}</select></label>`
      : '<p class="muted">No tienes instancias con Forge/Fabric/NeoForge/Quilt; crea una para usar la skin local.</p>';
    const grid = allSkins.length
      ? allSkins.map((s) => `
        <div class="item">
          ${skinCube(s.data)}
          <div class="grow"><h3 class="ellipsis">${esc(s.name)}</h3></div>
          <button class="btn small primary" data-act="off-skin" data-id="${esc(s.id)}">Poner skin</button>
        </div>`).join('')
      : '<p class="muted">Añade una skin arriba para usarla localmente.</p>';
    capesPart = `
      <div class="stack">
        ${sel}
        <div class="list">${grid}</div>
        <div class="row wrap"><button class="btn small" data-act="off-cape">Elegir capa (PNG)</button></div>
        <p class="muted" style="font-size:13px">La skin se coloca en la carpeta del mod CustomSkinLoader de la instancia elegida (se instala solo si falta) y la verás en partida y servidores sin verificación. Como se lee en local, esta skin solo es visible para ti y con este launcher.</p>
      </div>`;
  } else {
    capesPart = '<p class="muted">No se pudo leer el perfil de Minecraft.</p>';
  }
  showModal(`${sheetHead('Skins y capas')}
    <div class="stack">
      <div class="row wrap">
        <label class="field" style="max-width:190px"><span>Modelo</span><select id="w-variant"><option value="classic">Clásica</option><option value="slim">Delgada</option></select></label>
        <label class="field" style="max-width:150px"><span>Nombre</span><input type="text" id="w-name" maxlength="40" placeholder="opcional"></label>
        <label class="field" style="max-width:170px"><span>Buscar por nick</span><input type="text" id="w-nick" maxlength="20" placeholder="ejemplo: Notch"></label>
        <button class="btn" data-act="w-nick">${ic('search')}Buscar skin</button>
        <label class="field grow"><span>URL https de la skin</span><input type="text" id="w-url" placeholder="https://...png"></label>
        <button class="btn" data-act="w-add-url">Añadir por URL</button>
        <button class="btn" data-act="w-add-file">Elegir PNG</button>
      </div>
      <div class="list">${saved}</div>
      <h3 style="margin-top:6px">Capas de tu cuenta</h3>
      ${capesPart}
      <p class="muted" style="font-size:13px">Aplicar envía la skin elegida al perfil oficial de Minecraft; el cambio tarda un momento en aparecer en el juego.</p>
    </div>
    <footer><button class="btn" data-act="open-url" data-url="https://www.minecraft.net/es-es/profile">Perfil minecraft.net</button><button class="btn primary" data-act="close">Cerrar</button></footer>`, true);
  paintWardrobe();
}

async function pickFiles(filters, multiple) {
  try {
    const r = await dialog.open({ multiple, filters });
    if (r === null || r === undefined) return [];
    return Array.isArray(r) ? r : [r];
  } catch (_) {
    return [];
  }
}

async function startLogin() {
  let start;
  try {
    start = await api('ms_begin');
  } catch (_) {
    return;
  }
  S.loginActive = true;
  showModal(`${sheetHead('Iniciar sesión con Microsoft')}
    <div class="stack">
      <p>Abre el siguiente enlace en tu navegador y completa el inicio de sesión con Microsoft:</p>
      <div class="code" style="word-break:break-all">${esc(start.authorize_url)}</div>
      <div class="row" style="justify-content:center"><button class="btn primary" data-act="open-url" data-url="${esc(start.authorize_url)}">Abrir página de inicio de sesión</button></div>
      <p class="muted" style="text-align:center">Esperando a que completes el inicio de sesión…</p>
      <div class="bar indet"><i></i></div>
    </div>
    <footer><button class="btn" data-act="ms-cancel">Cancelar</button></footer>`);
  try {
    await api('ms_finish');
    await refreshAccounts();
    closeModal();
    toast('Sesión iniciada correctamente', 'ok');
    if (S.view === 'accounts') renderView();
  } catch (e) {
    if (S.loginActive) {
      closeModal();
      toast(typeof e === 'string' ? e : 'No se pudo iniciar sesión', 'err');
    }
  }
  S.loginActive = false;
}

async function refreshAccounts() {
  const res = await api('accounts_list');
  S.accounts = res.accounts;
  S.selectedAccount = res.selected;
  renderTop();
}

async function refreshInstances() {
  S.instances = await api('list_instances');
  if (!S.instances.some((i) => i.id === S.activeInstance)) S.activeInstance = S.instances[0] ? S.instances[0].id : null;
}

async function importPack(path) {
  try {
    const inst = await api('import_modpack_file', { path });
    await refreshInstances();
    S.activeInstance = inst.id;
    toast(`Modpack «${inst.name}» instalado como instancia`, 'ok');
    if (S.view === 'instances' || S.view === 'home') renderView();
  } catch (_) {}
  S.progress = null;
  updateDock();
}

function mdPlain(md) {
  return esc(String(md || '')
    .replace(/!\[[^\]]*\]\([^)]*\)/g, '')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/__([^_]+)__/g, '$1')
    .replace(/`([^`]*)`/g, '$1')
    .replace(/\n{3,}/g, '\n\n')
    .slice(0, 6000));
}

async function openDetails(id) {
  showModal(`${sheetHead('Detalles')}<div class="muted">Cargando…</div>`, true);
  try {
    const p = await api('project_details', { projectId: id });
    const cats = (p.categories || []).slice(0, 6).map((c) => `<span class="chip">${esc(c)}</span>`).join('');
    const gal = (p.gallery || []).slice(0, 12).map((u) => `<img src="${esc(u)}" alt="" loading="lazy">`).join('');
    showModal(`${sheetHead(esc(p.title || 'Detalles'))}
      <div class="stack">
        <div class="row" style="align-items:flex-start;gap:14px">
          ${p.icon ? `<img class="proj-icon" src="${esc(p.icon)}" alt="">` : '<div class="proj-icon"></div>'}
          <div class="stack" style="gap:6px;min-width:0">
            <p class="muted">${esc(p.description)}</p>
            <span class="muted">${fmtNum(p.downloads)} descargas · ${fmtNum(p.follows)} favoritos</span>
            ${cats ? `<div class="row wrap">${cats}</div>` : ''}
          </div>
        </div>
        ${gal ? `<h3>Galería</h3><div class="gallery">${gal}</div>` : ''}
        ${p.body ? `<h3>Descripción</h3><div class="detail-body">${mdPlain(p.body)}</div>` : ''}
      </div>
      <footer><button class="btn" data-act="close">Cerrar</button></footer>`, true);
  } catch (_) {}
}

async function importInstance(path) {
  try {
    toast('Importando instancia…');
    const inst = await api('import_instance_zip', { path });
    await refreshInstances();
    S.activeInstance = inst.id;
    toast(`Instancia «${inst.name}» importada`, 'ok');
    if (S.view === 'instances' || S.view === 'home') renderView();
  } catch (_) {}
}

async function importIntoContent(paths) {
  const t = S.content.tab;
  if (!S.content.open || !['mods', 'resourcepacks', 'shaderpacks'].includes(t)) return false;
  try {
    const n = await api('import_files', { id: S.content.id, kind: t, paths });
    toast(`${n} archivo(s)`, 'ok');
    openContent(S.content.id);
  } catch (_) {}
  return true;
}

async function handleDrop(paths) {
  if (!paths || !paths.length) return;
  const pack = paths.find((p) => p.toLowerCase().endsWith('.mrpack'));
  if (pack) return importPack(pack);
  if (await importIntoContent(paths)) return;
  const zip = paths.find((p) => p.toLowerCase().endsWith('.zip'));
  if (zip) return importInstance(zip);
}

function updateDock() {
  const dock = $('#dock');
  const p = S.progress;
  if (!p) {
    dock.hidden = true;
    return;
  }
  const pct = p.total > 0 ? Math.min(100, Math.round((p.done / p.total) * 100)) : 0;
  dock.hidden = false;
  const cancellable = ['download', 'modpack', 'java', 'loader', 'verify'].includes(p.stage);
  dock.innerHTML = tr(`<div class="row between"><b>${esc(p.label)}</b><span class="row"><span class="muted">${p.total > 0 ? `${p.done} / ${p.total}` : ''}</span>${cancellable ? '<button class="btn small" data-act="cancel-dl">Cancelar descarga</button>' : ''}</span></div><div class="bar ${p.total > 0 ? '' : 'indet'}"><i style="width:${p.total > 0 ? pct : 35}%"></i></div>`);
}

let dockTimer = null;
function onProgress(p) {
  clearTimeout(dockTimer);
  S.progress = p;
  updateDock();
  if (p.stage === 'running') playSound('start');
  if (p.stage === 'running' || (p.stage === 'modpack' && p.label === 'Modpack instalado')) {
    if (p.stage === 'modpack') playSound('done');
    dockTimer = setTimeout(() => {
      S.progress = null;
      updateDock();
    }, 1800);
  }
}

const actions = {
  nav: (d) => setView(d.view),
  close: () => closeModal(),
  pick: (d) => {
    S.activeInstance = d.id;
    renderView();
  },
  'new-instance': () => openCreate(),
  cloader: (d, el) => {
    el.parentElement.querySelectorAll('button').forEach((b) => b.classList.remove('on'));
    el.classList.add('on');
    loadLoaderVersions();
  },
  'create-submit': async () => {
    const name = $('#c-name').value.trim();
    if (!name) return toast('Escribe un nombre para la instancia', 'err');
    const loader = $('#c-loader .on').dataset.v;
    const btn = $('#c-submit');
    btn.disabled = true;
    btn.textContent = 'Creando…';
    try {
      const inst = await api('create_instance', { name, mcVersion: $('#c-version').value, loader, loaderVersion: loader === 'vanilla' ? null : $('#c-lv').value || null });
      await refreshInstances();
      S.activeInstance = inst.id;
      closeModal();
      toast('Instancia creada', 'ok');
      renderView();
    } catch (_) {
      btn.disabled = false;
      btn.textContent = 'Crear';
    }
  },
  edit: (d) => openEdit(d.id),
  accent: (d, el) => {
    S.editAccent = Number(d.n);
    el.parentElement.querySelectorAll('.swatch').forEach((b) => b.classList.toggle('on', b === el));
  },
  'edit-submit': async (d) => {
    const mem = parseInt($('#e-mem').value, 10);
    try {
      await api('update_instance', { id: d.id, patch: { name: $('#e-name').value, memory_mb: Number.isFinite(mem) ? mem : null, java_path: $('#e-java').value || null, jvm_args: $('#e-jvm').value || null, env_vars: $('#e-env').value || null, group: $('#e-group').value, accent: S.editAccent } });
      await refreshInstances();
      closeModal();
      renderView();
    } catch (_) {}
  },
  delete: (d) => {
    const i = S.instances.find((x) => x.id === d.id);
    showModal(`${sheetHead('Eliminar instancia')}<p>Se borrarán «${esc(i ? i.name : '')}», sus mundos y su configuración. No se puede deshacer.</p>
      <footer><button class="btn" data-act="close">Cancelar</button><button class="btn danger" data-act="delete-confirm" data-id="${esc(d.id)}">Eliminar</button></footer>`);
  },
  'delete-confirm': async (d) => {
    try {
      await api('delete_instance', { id: d.id });
      await refreshInstances();
      closeModal();
      renderView();
    } catch (_) {}
  },
  duplicate: async (d) => {
    try {
      await api('duplicate_instance', { id: d.id });
      await refreshInstances();
      toast('Instancia creada', 'ok');
      renderView();
    } catch (_) {}
  },
  export: async (d) => {
    const inst = S.instances.find((i) => i.id === d.id);
    try {
      const dest = await dialog.save({ defaultPath: `${inst ? inst.name : 'modpack'}.mrpack`, filters: [{ name: 'Modrinth modpack', extensions: ['mrpack'] }] });
      if (!dest) return;
      toast('Exportando…');
      await api('export_modpack', { id: d.id, dest });
      toast('Exportado', 'ok');
    } catch (_) {}
  },
  'import-pack': async () => {
    const files = await pickFiles([{ name: 'Modrinth modpack', extensions: ['mrpack'] }], false);
    if (files.length) await importPack(files[0]);
  },
  'import-instance': async () => {
    const files = await pickFiles([{ name: 'Instancia de Prism/MultiMC', extensions: ['zip'] }], false);
    if (files.length) await importInstance(files[0]);
  },
  'add-files': async () => {
    const t = S.content.tab;
    const ext = t === 'mods' ? 'jar' : 'zip';
    const files = await pickFiles([{ name: ext.toUpperCase(), extensions: [ext] }], true);
    if (files.length) await importIntoContent(files);
  },
  'check-updates': async () => {
    try {
      S.content.updates = await api('check_mod_updates', { id: S.content.id });
      openContent(S.content.id);
    } catch (_) {}
  },
  'apply-update': async (d) => {
    try {
      await api('apply_mod_update', { id: S.content.id, oldName: d.name, versionId: d.vid });
      S.content.updates = (S.content.updates || []).filter((u) => u.file !== d.name);
      openContent(S.content.id);
    } catch (_) {}
  },
  'apply-all': async () => {
    const list = [...(S.content.updates || [])];
    for (const u of list) {
      try {
        await api('apply_mod_update', { id: S.content.id, oldName: u.file, versionId: u.version_id });
        S.content.updates = S.content.updates.filter((x) => x.file !== u.file);
      } catch (_) {}
    }
    openContent(S.content.id);
  },
  backup: async () => {
    try {
      await api('backup_worlds', { id: S.content.id });
      toast('Copia creada', 'ok');
      openContent(S.content.id);
    } catch (_) {}
  },
  restore: async (d) => {
    try {
      await api('restore_backup', { id: S.content.id, name: d.name });
      toast('Copia restaurada', 'ok');
    } catch (_) {}
  },
  'sv-add': async () => {
    const inst = S.instances.find((i) => i.id === S.content.id);
    const name = $('#sv-name').value.trim();
    const address = $('#sv-addr').value.trim();
    if (!inst || !name || !address) return;
    try {
      await api('set_servers', { id: inst.id, servers: [...(inst.servers || []), { name, address }] });
      await refreshInstances();
      toast('Servidor añadido', 'ok');
      openContent(S.content.id, 'servers');
    } catch (_) {}
  },
  'sv-del': async (d) => {
    const inst = S.instances.find((i) => i.id === S.content.id);
    if (!inst) return;
    try {
      await api('set_servers', { id: inst.id, servers: (inst.servers || []).filter((_, n) => n !== Number(d.n)) });
      await refreshInstances();
      openContent(S.content.id, 'servers');
    } catch (_) {}
  },
  'play-server': (d) => {
    closeModal();
    actions.play({ id: S.content.id, server: d.address });
  },
  wardrobe: (d) => openWardrobe(d.id),
  'w-add-url': async () => {
    const url = $('#w-url') ? $('#w-url').value.trim() : '';
    if (!url.startsWith('https://')) return toast('Pega una URL https que apunte al PNG de la skin', 'err');
    try {
      await api('wardrobe_add', { source: url, name: $('#w-name') ? $('#w-name').value : '' });
      toast('Skin guardada en el armario', 'ok');
      openWardrobe(wardrobeAcc);
    } catch (_) {}
  },
  'w-nick': async () => {
    const nick = $('#w-nick') ? $('#w-nick').value.trim() : '';
    if (!nick) return toast('Escribe un nick para buscar');
    try {
      const url = await api('skin_lookup', { name: nick });
      await api('wardrobe_add', { source: url, name: nick });
      toast(`Skin de ${nick} guardada en el armario`, 'ok');
      openWardrobe(wardrobeAcc);
    } catch (e) {
      toast(String(e).replace(/^Error: /, ''), 'err');
    }
  },
  'w-add-file': async () => {
    const files = await pickFiles([{ name: 'PNG', extensions: ['png'] }], false);
    if (!files.length) return;
    try {
      await api('wardrobe_add', { source: files[0], name: $('#w-name') ? $('#w-name').value : '' });
      toast('Skin guardada en el armario', 'ok');
      openWardrobe(wardrobeAcc);
    } catch (_) {}
  },
  'wardrobe-apply': async (d) => {
    const variant = $('#w-variant') ? $('#w-variant').value : 'classic';
    try {
      await api('wardrobe_apply', { accountId: wardrobeAcc, skinId: d.id, variant });
      toast('Skin aplicada a tu cuenta', 'ok');
    } catch (_) {}
  },
  'wardrobe-remove': async (d) => {
    try {
      await api('wardrobe_remove', { id: d.id });
      openWardrobe(wardrobeAcc);
    } catch (_) {}
  },
  'cape-on': async (d) => {
    try {
      await api('cape_set', { accountId: wardrobeAcc, capeId: d.id });
      toast('Capa activada', 'ok');
      openWardrobe(wardrobeAcc);
    } catch (_) {}
  },
  'cape-clear': async () => {
    try {
      await api('cape_set', { accountId: wardrobeAcc, capeId: null });
      toast('Capa quitada', 'ok');
      openWardrobe(wardrobeAcc);
    } catch (_) {}
  },
  'off-skin': async (d) => {
    const inst = $('#w-off-inst') ? $('#w-off-inst').value : '';
    if (!inst) return toast('Crea una instancia con Fabric, Forge, NeoForge o Quilt', 'err');
    const acc = S.accounts.find((a) => a.id === wardrobeAcc);
    try {
      await api('offline_skin_apply', { instanceId: inst, player: acc ? acc.name : '', skinId: d.id });
      toast('Skin colocada en la instancia (CustomSkinLoader)', 'ok');
    } catch (_) {}
  },
  'off-cape': async () => {
    const inst = $('#w-off-inst') ? $('#w-off-inst').value : '';
    if (!inst) return toast('Crea una instancia con Fabric, Forge, NeoForge o Quilt', 'err');
    const files = await pickFiles([{ name: 'PNG', extensions: ['png'] }], false);
    if (!files.length) return;
    const acc = S.accounts.find((a) => a.id === wardrobeAcc);
    try {
      await api('offline_cape_apply', { instanceId: inst, player: acc ? acc.name : '', path: files[0] });
      toast('Capa colocada en la instancia', 'ok');
    } catch (_) {}
  },
  'copy-log': async () => {
    try {
      await navigator.clipboard.writeText(S.lastCrash || '');
      toast('Registro copiado', 'ok');
    } catch (_) {}
  },
  'cancel-dl': () => api('cancel_downloads', {}, true).catch(() => {}),
  details: (d) => openDetails(d.id),
  optimize: async (d, el) => {
    el.disabled = true;
    el.textContent = 'Optimizando…';
    try {
      const files = await api('optimize_instance', { id: d.id });
      toast(files.length ? `Instancia optimizada: ${files.length} mod(s) de rendimiento` : 'Instancia optimizada', 'ok');
    } catch (_) {}
    openContent(d.id, 'mods');
  },
  verify: async (d, el) => {
    el.disabled = true;
    try {
      const r = await api('verify_instance', { id: d.id });
      toast(r.repaired > 0 ? `${r.repaired} de ${r.checked} archivos reparados` : `${r.checked} archivos verificados, todo en orden`, 'ok');
    } catch (_) {}
    el.disabled = false;
  },
  'icon-pick': async (d) => {
    const files = await pickFiles([{ name: 'PNG', extensions: ['png'] }], false);
    if (!files.length) return;
    try {
      await api('set_instance_icon', { id: d.id, path: files[0] });
      await refreshInstances();
      openEdit(d.id);
      toast('Icono actualizado', 'ok');
      renderView();
    } catch (_) {}
  },
  'icon-remove': async (d) => {
    try {
      await api('remove_instance_icon', { id: d.id });
      await refreshInstances();
      openEdit(d.id);
      renderView();
    } catch (_) {}
  },
  'sv-refresh': () => {
    const inst = S.instances.find((i) => i.id === S.content.id);
    if (inst && inst.servers) fetchServerStatuses(inst.servers);
  },
  folder: (d) => api('open_folder', { id: d.id, sub: d.sub || null }).catch(() => {}),
  'folder-root': () => api('open_folder', { id: null, sub: null }).catch(() => {}),
  content: (d) => openContent(d.id, 'mods'),
  ctab: (d) => openContent(S.content.id, d.tab),
  ctoggle: async (d) => {
    try {
      await api('toggle_content', { id: S.content.id, kind: S.content.tab, name: d.name });
      openContent(S.content.id);
    } catch (_) {}
  },
  cremove: async (d) => {
    try {
      await api('remove_content', { id: S.content.id, kind: S.content.tab, name: d.name });
      openContent(S.content.id);
    } catch (_) {}
  },
  play: async (d) => {
    if (!activeAccount()) {
      toast('Añade una cuenta para jugar', 'err');
      return setView('accounts');
    }
    S.progress = { stage: 'prepare', label: 'Preparando…', done: 0, total: 0 };
    updateDock();
    S.logs.push(`[Nebula] Iniciando ${d.id}`);
    try {
      await api('launch', { instanceId: d.id, accountId: S.selectedAccount, server: d.server || null });
      S.running.add(d.id);
      if (S.view === 'home' || S.view === 'instances') renderView();
    } catch (_) {
      S.progress = null;
      updateDock();
    }
  },
  stop: (d) => api('stop', { instanceId: d.id }).catch(() => {}),
  dkind: (d) => {
    S.discover.kind = d.kind;
    S.discover.items = [];
    S.discover.error = '';
    renderView();
    runSearch(true);
  },
  dmore: () => runSearch(false),
  install: async (d, el) => {
    const kind = S.discover.kind;
    el.disabled = true;
    try {
      if (kind === 'modpack') {
        const inst = await api('install_modpack', { projectId: d.id, title: d.title });
        await refreshInstances();
        S.activeInstance = inst.id;
        toast(`Modpack «${inst.name}» instalado como instancia`, 'ok');
      } else {
        const target = $('#d-target') ? $('#d-target').value : '';
        if (!target) {
          toast('Crea o elige una instancia de destino', 'err');
          return;
        }
        const files = await api('install_project', { kind, projectId: d.id, instanceId: target });
        toast(files.length > 1 ? `Instalado con ${files.length - 1} dependencia(s)` : `Instalado: ${files[0]}`, 'ok');
      }
    } catch (_) {
    } finally {
      el.disabled = false;
      if (kind === 'modpack') {
        S.progress = null;
        updateDock();
      }
    }
  },
  'add-offline': async () => {
    try {
      await api('add_offline', { name: $('#offline-name').value });
      await refreshAccounts();
      renderView();
    } catch (_) {}
  },
  'ms-login': () => startLogin(),
  'ms-cancel': () => {
    S.loginActive = false;
    api('ms_cancel', {}, true).catch(() => {});
    closeModal();
  },
  'select-account': async (d) => {
    await api('select_account', { id: d.id });
    await refreshAccounts();
    renderView();
  },
  'remove-account': async (d) => {
    await api('remove_account', { id: d.id });
    await refreshAccounts();
    renderView();
  },
  'open-url': (d) => api('open_url', { url: d.url }).catch(() => {}),
  'update-go': (d) => {
    toast('Buscando actualización…', 'ok');
    api('self_update').then((m) => toast(m, 'ok')).catch(async () => {
      try { await api('open_url', { url: d.url }); } catch (_) { toast('No se pudo abrir el navegador', 'err'); }
    });
  },
  'jvm-preset': (d) => {
    const ta = document.getElementById(d.target);
    if (ta) {
      ta.value = JVM_PRESETS[d.preset] || '';
      toast('Preset colocado; pulsa Guardar para aplicarlo', 'ok');
    }
  },
  'shot-icon': async (d) => {
    try {
      await api('screenshot_to_icon', { id: d.id, name: d.name });
      S.instances = await api('list_instances') || [];
      toast('Captura puesta como icono de la instancia', 'ok');
      renderView();
    } catch (e) {
      toast(String(e), 'err');
    }
  },
  'mp-refresh': async (d) => {
    if (!S.progress) S.progress = { kind: 'modpack' };
    renderTop();
    try {
      await api('refresh_modpack', { id: d.id });
      toast('Modpack actualizado', 'ok');
      S.instances = await api('list_instances') || [];
    } catch (e) {
      toast(String(e).replace(/^Error: /, ''), 'err');
    }
    S.progress = null;
    renderTop();
  },
  'srv-popular': () => openPopular(),
  'pop-add': async (d) => {
    const id = S.content.id;
    if (!id) return;
    try {
      const cur = S.instances.find((i) => i.id === id);
      const servers = (cur && cur.servers ? cur.servers : []).map((s) => ({ name: s.name, address: s.address }));
      if (servers.some((s) => s.address === d.addr)) return toast('Ese servidor ya está en la lista');
      servers.push({ name: d.name, address: d.addr });
      const upd = await api('set_servers', { id, servers });
      const ix = S.instances.findIndex((i) => i.id === id);
      if (ix >= 0) S.instances[ix] = upd;
      toast(`Servidor ${d.name} añadido`, 'ok');
      openContent(id, 'servers');
    } catch (e) {
      toast(String(e), 'err');
    }
  },
  'about': async () => {
    let info = { name: 'Nebula Launcher', version: '', repo: 'https://github.com/mclovin552/nebula-launcher', issues: 'https://github.com/mclovin552/nebula-launcher/issues' };
    try {
      info = { ...info, ...(await api('app_info')) };
    } catch (_) {}
    showModal(`${sheetHead('Acerca de Nebula')}
      <div class="stack" style="text-align:center;padding:10px">
        <h2>☄ ${esc(info.name)}</h2>
        <p class="muted">Versión ${esc(info.version)}</p>
        <p class="muted" style="max-width:420px;margin:0 auto">Lanzador de Minecraft Java de código abierto creado por mclovin552. Licencia MIT: puedes usarlo, copiarlo y mejorarlo libremente.</p>
      </div>
      <footer><button class="btn" data-act="open-url" data-url="${esc(info.issues)}">Reportar un fallo</button><button class="btn primary" data-act="open-url" data-url="${esc(info.repo)}">Ver en GitHub</button></footer>`);
  },
  'clear-logs': () => {
    S.logs = [];
    renderView();
  },
  'save-settings': async () => {
    const s = {
      ...S.settings,
      max_memory: parseInt($('#s-max').value, 10),
      min_memory: Math.min(parseInt($('#s-min').value, 10), parseInt($('#s-max').value, 10)),
      java_path: $('#s-java').value.trim(),
      jvm_args: $('#s-jvm').value.trim(),
      wrapper: $('#s-wrap').value.trim(),
      width: parseInt($('#s-w').value, 10) || 1280,
      height: parseInt($('#s-h').value, 10) || 720,
      concurrency: Math.max(1, Math.min(128, parseInt($('#s-conc').value, 10) || 32)),
      env_vars: $('#s-env').value,
      pre_launch: $('#s-pre').value.trim(),
      post_exit: $('#s-post').value.trim(),
      language: $('#s-lang').value,
      theme: $('#s-theme') ? $('#s-theme').value : 'dark',
      update_repo: $('#s-repo').value.trim(),
      close_to_tray: $('#s-tray') ? $('#s-tray').checked : true,
      auto_backup_minutes: Math.max(0, Math.min(240, parseInt(($('#s-backup') || {}).value, 10) || 0)),
      discord_app_id: (($('#s-discord') || {}).value || '').trim().replace(/[^0-9]/g, '')
    };
    try {
      await api('settings_set', { settings: s });
      S.settings = s;
      applyTheme();
      renderRail();
      renderTop();
      renderView();
      toast('Ajustes guardados', 'ok');
    } catch (_) {}
  }
};

document.addEventListener('click', (e) => {  const el = e.target.closest('[data-act]');
  if (!el) {
    if (e.target.id === 'modal') closeModal();
    return;
  }
  const fn = actions[el.dataset.act];
  if (fn) fn(el.dataset, el);
});

let dragInst = null;
document.addEventListener('dragstart', (e) => {
  const c = e.target.closest && e.target.closest('.card.with-cover');
  if (c && c.dataset.id && S.view === 'instances') {
    dragInst = c.dataset.id;
    e.dataTransfer.effectAllowed = 'move';
  }
});
document.addEventListener('dragend', () => { dragInst = null; });
document.addEventListener('dragover', (e) => {
  if (dragInst && S.view === 'instances') e.preventDefault();
});
document.addEventListener('drop', async (e) => {
  if (!dragInst || S.view !== 'instances') return;
  e.preventDefault();
  const c = e.target.closest && e.target.closest('.card.with-cover');
  if (c && c.dataset.id && c.dataset.id !== dragInst) {
    try {
      await api('reorder_instances', { id: dragInst, other: c.dataset.id });
      S.instances = await api('list_instances') || [];
      renderView();
      toast('Orden cambiado', 'ok');
    } catch (_) {}
  }
  dragInst = null;
});

document.addEventListener('input', (e) => {
  if (e.target.id === 'd-query') {
    S.discover.query = e.target.value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => runSearch(true), 350);
  }
});

const viewEl = () => document.getElementById('view');
if (viewEl()) {
  viewEl().addEventListener('scroll', () => {
    const v = viewEl();
    const bar = document.getElementById('topbar');
    if (bar) bar.classList.toggle('scrolled', v.scrollTop > 8);
    const top = document.getElementById('to-top');
    if (top) top.classList.toggle('show', v.scrollTop > 420);
  });
  document.addEventListener('click', (e) => {
    if (e.target.id === 'to-top' || (e.target.closest && e.target.closest('#to-top'))) {
      viewEl().scrollTo({ top: 0, behavior: 'smooth' });
    }
  });
}

document.addEventListener('change', (e) => {
  if (e.target.id === 'd-sort') {
    S.discover.sort = e.target.value;
    runSearch(true);
  }
  if (e.target.id === 'd-target') {
    S.discover.target = e.target.value;
    runSearch(true);
  }
});

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && !$('#modal').hidden) closeModal();
  if (e.key === 'Enter' && e.target.id === 'offline-name') actions['add-offline']();
});

window.addEventListener('resize', () => {
  if (S.view === 'home') drawHero();
});

async function boot() {
  renderRail();
  renderTop();
  await listen('progress', (e) => onProgress(e.payload));
  await listen('game-log', (e) => {
    S.logs.push(e.payload.line);
    if (S.logs.length > 3000) S.logs.splice(0, S.logs.length - 3000);
    if (S.view === 'console') {
      const c = $('#console');
      if (c) {
        const stick = c.scrollTop + c.clientHeight >= c.scrollHeight - 40;
        c.insertAdjacentHTML('beforeend', logHtml(e.payload.line));
        if (stick) c.scrollTop = c.scrollHeight;
      }
    }
  });
  await listen('game-exit', (e) => {
    S.running.delete(e.payload.instance);
    S.logs.push(`[Nebula] El juego terminó con código ${e.payload.code}`);
    const normal = [0, null, 130, 143].includes(e.payload.code);
    if (normal) playSound('exit'); else playSound('err');
    if (!normal) showCrash(e.payload.instance);
    if (S.view === 'home' || S.view === 'instances') renderView();
  });
  await listen('tauri://drag-drop', (e) => handleDrop(e.payload && e.payload.paths));
  try {
    const [settings, sys] = await Promise.all([api('settings_get'), api('system_info')]);
    S.settings = settings;
    S.sys = sys;
    applyTheme();
    await Promise.all([refreshInstances(), refreshAccounts()]);
    const running = await api('running', {}, true);
    S.running = new Set(running);
  } catch (_) {
    S.settings = S.settings || { min_memory: 1024, max_memory: 4096, java_path: '', jvm_args: '', wrapper: '', width: 1280, height: 720, concurrency: 32, show_snapshots: false, theme: 'dark' };
  }
  S.discover.target = S.activeInstance || '';
  renderRail();
  renderTop();
  renderView();
  api('check_launcher_update', {}, true).then((u) => {
    if (u && u.available) {
      S.updateAv = u;
      renderTop();
    }
  }).catch(() => {});
  api('versions', {}, true).then((v) => (S.versions = v)).catch(() => {});
}

boot();
