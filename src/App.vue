<script setup lang="ts">
import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

const GAME_PATH_KEY = 'sts2mod.gamePath';
const LIB_PATH_KEY = 'sts2mod.libPath';
const DEFAULT_GAME_PATH = "C:\\SteamLibrary\\steamapps\\common\\SlayTheSpire2";
const DEFAULT_LIB_PATH = "D:\\STS2_Mods";

const readStoredPath = (key: string, fallback: string): string => {
  const value = localStorage.getItem(key);
  if (!value || !value.trim()) return fallback;
  return value;
};

const gamePath = ref(readStoredPath(GAME_PATH_KEY, DEFAULT_GAME_PATH));
const libPath = ref(readStoredPath(LIB_PATH_KEY, DEFAULT_LIB_PATH)); // 确保这个文件夹里有几个子文件夹
const pckPath = ref("");
const dllPath = ref("");
const folderPath = ref("");
const modName = ref("");
const isDropActive = ref(false);
const mods = ref<{ id: string; enabled: boolean }[]>([]);
const selectedModIds = ref<string[]>([]);
const errorMsg = ref("");
const addMsg = ref("");

watch(gamePath, (value) => {
  localStorage.setItem(GAME_PATH_KEY, value);
});

watch(libPath, (value) => {
  localStorage.setItem(LIB_PATH_KEY, value);
});

const scan = async () => {
  errorMsg.value = "";
  try {
    const result: any[] = await invoke('get_mods', {
      libraryPath: libPath.value,
      gameModsPath: `${gamePath.value}\\mods`
    });
    mods.value = result;
    selectedModIds.value = selectedModIds.value.filter((id) =>
      result.some((mod) => mod.id === id)
    );
    if (result.length === 0) errorMsg.value = "仓库文件夹内未发现 Mod 子目录。";
  } catch (err: any) {
    errorMsg.value = err;
    mods.value = [];
    selectedModIds.value = [];
  }
};

const handleToggle = async (mod: any) => {
  try {
    await invoke('toggle_mod', {
      modId: mod.id,
      libraryPath: libPath.value,
      gameModsPath: `${gamePath.value}\\mods`,
      enable: !mod.enabled
    });
    mod.enabled = !mod.enabled;
  } catch (err) {
    alert("操作失败: " + err);
  }
};

const normalizeSinglePath = (picked: string | string[] | null): string => {
  if (!picked) return "";
  if (Array.isArray(picked)) return picked[0] ?? "";
  return picked;
};

const pickPckFile = async () => {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'PCK', extensions: ['pck'] }]
  });
  const path = normalizeSinglePath(selected);
  if (path) pckPath.value = path;
};

const pickDllFile = async () => {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'DLL', extensions: ['dll'] }]
  });
  const path = normalizeSinglePath(selected);
  if (path) dllPath.value = path;
};

const pickModFolder = async () => {
  const selected = await open({
    multiple: false,
    directory: true
  });
  const path = normalizeSinglePath(selected);
  if (path) folderPath.value = path;
};

const parseDroppedPaths = (files: FileList): string[] => {
  const paths: string[] = [];
  for (let i = 0; i < files.length; i += 1) {
    const file = files.item(i) as (File & { path?: string }) | null;
    if (!file) continue;
    if (file.path && file.path.trim()) {
      paths.push(file.path);
    }
  }
  return paths;
};

const fillPathsFromDropped = (paths: string[]) => {
  for (const path of paths) {
    const lower = path.toLowerCase();
    if (lower.endsWith('.pck') && !pckPath.value) {
      pckPath.value = path;
    } else if (lower.endsWith('.dll') && !dllPath.value) {
      dllPath.value = path;
    }
  }
};

const onDrop = (event: DragEvent) => {
  event.preventDefault();
  isDropActive.value = false;
  const dropped = event.dataTransfer?.files;
  if (!dropped || dropped.length === 0) return;
  const paths = parseDroppedPaths(dropped);
  if (paths.length === 0) {
    addMsg.value = '拖拽失败：请从系统文件管理器拖入 .pck/.dll 文件。';
    return;
  }
  fillPathsFromDropped(paths);
};

const onDragOver = (event: DragEvent) => {
  event.preventDefault();
  isDropActive.value = true;
};

const onDragLeave = (event: DragEvent) => {
  event.preventDefault();
  isDropActive.value = false;
};

const addMod = async () => {
  addMsg.value = "";
  errorMsg.value = "";
  if (!pckPath.value.trim() || !dllPath.value.trim()) {
    addMsg.value = "请同时输入 .pck 和 .dll 文件路径。";
    return;
  }
  try {
    const importedModName = await invoke<string>('add_mod_from_files', {
      pckPath: pckPath.value.trim(),
      dllPath: dllPath.value.trim(),
      libraryPath: libPath.value,
      modName: modName.value.trim() || null
    });
    addMsg.value = `已添加 Mod: ${importedModName}`;
    await scan();
  } catch (err: any) {
    addMsg.value = `添加失败: ${String(err)}`;
  }
};

const addModFromFolder = async () => {
  addMsg.value = "";
  errorMsg.value = "";
  if (!folderPath.value.trim()) {
    addMsg.value = "请先选择 Mod 文件夹路径。";
    return;
  }
  try {
    const importedModName = await invoke<string>('add_mod_from_folder', {
      sourcePath: folderPath.value.trim(),
      libraryPath: libPath.value,
      modName: modName.value.trim() || null
    });
    addMsg.value = `已添加 Mod 文件夹: ${importedModName}`;
    await scan();
  } catch (err: any) {
    addMsg.value = `添加失败: ${String(err)}`;
  }
};

const isSelected = (modId: string): boolean => {
  return selectedModIds.value.includes(modId);
};

const toggleSelected = (modId: string, checked: boolean) => {
  if (checked) {
    if (!selectedModIds.value.includes(modId)) {
      selectedModIds.value.push(modId);
    }
  } else {
    selectedModIds.value = selectedModIds.value.filter((id) => id !== modId);
  }
};

const selectAllMods = () => {
  selectedModIds.value = mods.value.map((mod) => mod.id);
};

const clearSelectedMods = () => {
  selectedModIds.value = [];
};

const batchToggleMods = async (enable: boolean) => {
  if (selectedModIds.value.length === 0) {
    addMsg.value = "请先勾选至少一个 Mod。";
    return;
  }

  addMsg.value = "";
  errorMsg.value = "";

  const selectedSet = new Set(selectedModIds.value);
  const targetMods = mods.value.filter((mod) => selectedSet.has(mod.id));

  try {
    for (const mod of targetMods) {
      if (mod.enabled === enable) {
        continue;
      }
      await invoke('toggle_mod', {
        modId: mod.id,
        libraryPath: libPath.value,
        gameModsPath: `${gamePath.value}\\mods`,
        enable
      });
      mod.enabled = enable;
    }
    addMsg.value = enable
      ? `已批量启用 ${targetMods.length} 个 Mod。`
      : `已批量禁用 ${targetMods.length} 个 Mod。`;
  } catch (err: any) {
    addMsg.value = `批量操作失败: ${String(err)}`;
  }
};
</script>

<template>
  <div class="p-8 max-w-4xl mx-auto">
    <h1 class="text-3xl font-black text-red-500 mb-6 italic">SPIRE MGR 2</h1>
    <div class="bg-slate-800 p-6 rounded-xl mb-8 border border-slate-700 shadow-xl">
      <div class="space-y-4 text-sm">
        <div class="flex items-center gap-4">
          <label class="w-32 text-slate-400 font-bold">游戏主目录:</label>
          <input v-model="gamePath" class="flex-1 bg-slate-900 border border-slate-600 p-2 rounded text-red-300 font-mono" />
        </div>
        <div class="flex items-center gap-4">
          <label class="w-32 text-slate-400 font-bold">Mod 仓库:</label>
          <input v-model="libPath" class="flex-1 bg-slate-900 border border-slate-600 p-2 rounded text-red-300 font-mono" />
        </div>
        <div class="flex items-center gap-4">
          <label class="w-32 text-slate-400 font-bold">PCK 文件:</label>
          <input v-model="pckPath" class="flex-1 bg-slate-900 border border-slate-600 p-2 rounded text-red-300 font-mono" placeholder="例如: D:\\Downloads\\SomeMod\\MyMod.pck" />
          <button @click="pickPckFile" class="px-4 py-2 rounded bg-slate-700 hover:bg-slate-600 font-bold whitespace-nowrap">
            选择文件
          </button>
        </div>
        <div class="flex items-center gap-4">
          <label class="w-32 text-slate-400 font-bold">DLL 文件:</label>
          <input v-model="dllPath" class="flex-1 bg-slate-900 border border-slate-600 p-2 rounded text-red-300 font-mono" placeholder="例如: D:\\Downloads\\SomeMod\\MyMod.dll" />
          <button @click="pickDllFile" class="px-4 py-2 rounded bg-slate-700 hover:bg-slate-600 font-bold whitespace-nowrap">
            选择文件
          </button>
        </div>
        <div class="flex items-center gap-4">
          <label class="w-32 text-slate-400 font-bold">Mod 文件夹:</label>
          <input v-model="folderPath" class="flex-1 bg-slate-900 border border-slate-600 p-2 rounded text-red-300 font-mono" placeholder="例如: D:\\Downloads\\SomeModFolder" />
          <button @click="pickModFolder" class="px-4 py-2 rounded bg-slate-700 hover:bg-slate-600 font-bold whitespace-nowrap">
            选择文件夹
          </button>
        </div>
        <div
          @drop="onDrop"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          :class="isDropActive ? 'border-emerald-400 bg-emerald-500/10' : 'border-slate-600 bg-slate-900/60'"
          class="border-2 border-dashed rounded-lg p-4 text-center transition-colors"
        >
          把 .pck 和 .dll 拖到这里可自动填充路径
        </div>
        <div class="flex items-center gap-4">
          <label class="w-32 text-slate-400 font-bold">Mod 名称:</label>
          <input v-model="modName" class="flex-1 bg-slate-900 border border-slate-600 p-2 rounded text-red-300 font-mono" placeholder="可选，不填则用 pck 文件名" />
        </div>
        <button @click="addMod" class="w-full bg-emerald-600 hover:bg-emerald-500 py-3 rounded font-black transition-all shadow-lg active:scale-95">
          添加 .pck + .dll 到仓库
        </button>
        <button @click="addModFromFolder" class="w-full bg-cyan-700 hover:bg-cyan-600 py-3 rounded font-black transition-all shadow-lg active:scale-95">
          添加 Mod 文件夹到仓库
        </button>
        <p v-if="addMsg" class="text-cyan-300 bg-cyan-500/10 p-3 rounded text-center border border-cyan-500/20">
          {{ addMsg }}
        </p>
        <button @click="scan" class="w-full bg-red-600 hover:bg-red-500 py-3 rounded font-black transition-all shadow-lg active:scale-95">
          立即扫描并刷新列表
        </button>
      </div>
    </div>
    <p v-if="errorMsg" class="text-yellow-500 bg-yellow-500/10 p-3 rounded mb-4 text-center border border-yellow-500/20">
      ⚠️ {{ errorMsg }}
    </p>
    <div class="bg-slate-800 border border-slate-700 p-4 rounded-lg mb-4 flex flex-wrap items-center gap-2">
      <button @click="selectAllMods" class="px-3 py-2 rounded bg-slate-700 hover:bg-slate-600 font-bold text-sm">
        全选
      </button>
      <button @click="clearSelectedMods" class="px-3 py-2 rounded bg-slate-700 hover:bg-slate-600 font-bold text-sm">
        清空
      </button>
      <button
        @click="batchToggleMods(true)"
        :disabled="selectedModIds.length === 0"
        class="px-3 py-2 rounded bg-emerald-700 hover:bg-emerald-600 disabled:opacity-40 disabled:cursor-not-allowed font-bold text-sm"
      >
        批量启用
      </button>
      <button
        @click="batchToggleMods(false)"
        :disabled="selectedModIds.length === 0"
        class="px-3 py-2 rounded bg-amber-700 hover:bg-amber-600 disabled:opacity-40 disabled:cursor-not-allowed font-bold text-sm"
      >
        批量禁用
      </button>
      <span class="ml-auto text-slate-400 text-sm">已选 {{ selectedModIds.length }} 个</span>
    </div>
    <div class="bg-slate-800 border border-slate-700 rounded-lg overflow-hidden divide-y divide-slate-700">
      <div
        v-for="mod in mods"
        :key="mod.id"
        class="h-14 px-4 flex flex-nowrap justify-between items-center gap-3 transition-colors hover:bg-slate-750/60"
      >
        <div class="flex items-center gap-3 min-w-0 flex-1 overflow-hidden">
          <input
            type="checkbox"
            :checked="isSelected(mod.id)"
            @change="toggleSelected(mod.id, ($event.target as HTMLInputElement).checked)"
            class="w-4 h-4 shrink-0"
          />
          <h3 class="text-base font-bold leading-none truncate whitespace-nowrap">{{ mod.id }}</h3>
        </div>
        <button
          @click="handleToggle(mod)"
          :class="mod.enabled ? 'bg-red-600' : 'bg-slate-600'"
          class="px-5 py-1.5 rounded-full font-black text-xs uppercase tracking-widest transition-all shadow-md shrink-0 whitespace-nowrap"
        >
          {{ mod.enabled ? 'Enabled' : 'Disabled' }}
        </button>
      </div>
    </div>
  </div>
</template>