#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::fs;
use std::path::{Path, PathBuf};

// 定义返回给前端的 Mod 信息结构
#[derive(serde::Serialize)]
struct ModInfo {
    id: String,
    enabled: bool,
}

fn find_mod_pair_files(mod_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let entries = fs::read_dir(mod_dir).map_err(|e| e.to_string())?;
    let mut pck_file: Option<PathBuf> = None;
    let mut dll_file: Option<PathBuf> = None;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "pck" && pck_file.is_none() {
            pck_file = Some(path.clone());
        } else if ext == "dll" && dll_file.is_none() {
            dll_file = Some(path.clone());
        }
    }

    let pck = pck_file.ok_or_else(|| "Mod 目录中未找到 .pck 文件".to_string())?;
    let dll = dll_file.ok_or_else(|| "Mod 目录中未找到 .dll 文件".to_string())?;
    Ok((pck, dll))
}

fn list_mod_root_files(mod_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(mod_dir).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }

    let has_pck = files.iter().any(|p| {
        p.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .eq_ignore_ascii_case("pck")
    });
    let has_dll = files.iter().any(|p| {
        p.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .eq_ignore_ascii_case("dll")
    });

    if !has_pck {
        return Err("Mod 目录中未找到 .pck 文件".into());
    }
    if !has_dll {
        return Err("Mod 目录中未找到 .dll 文件".into());
    }

    Ok(files)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), String> {
    if !target.exists() {
        fs::create_dir_all(target).map_err(|e| e.to_string())?;
    }

    let entries = fs::read_dir(source).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = target.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn target_points_to_source(target: &Path, source: &Path) -> bool {
    let source_real = fs::canonicalize(source).ok();
    let target_real = fs::canonicalize(target).ok();
    source_real.is_some() && source_real == target_real
}

fn ensure_link_or_empty(target: &Path, source: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }
    if target_points_to_source(target, source) {
        return Ok(());
    }
    Err(format!("游戏 Mod 目录已有同名文件: {}", target.display()))
}

fn remove_link_if_owned(target: &Path, source: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(target).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        if target_points_to_source(target, source) || fs::canonicalize(target).is_err() {
            fs::remove_file(target).map_err(|e| e.to_string())?;
            return Ok(());
        }
        return Err(format!("同名链接不属于当前 Mod: {}", target.display()));
    }

    Err(format!("检测到同名实体文件，已停止删除: {}", target.display()))
}

#[tauri::command]
fn get_mods(library_path: String, game_mods_path: String) -> Result<Vec<ModInfo>, String> {
    let lib_p = Path::new(&library_path);
    if !lib_p.exists() {
        return Err("Mod 仓库路径不存在".into());
    }

    if !Path::new(&game_mods_path).exists() {
        fs::create_dir_all(&game_mods_path).map_err(|e| e.to_string())?;
    }

    let mut mods = Vec::new();
    let entries = fs::read_dir(lib_p).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let mod_dir = entry.path();
        if mod_dir.is_dir() {
            let mod_id = entry.file_name().to_string_lossy().into_owned();
            let Ok((pck_source, dll_source)) = find_mod_pair_files(&mod_dir) else {
                continue;
            };
            let pck_name = pck_source
                .file_name()
                .ok_or_else(|| "无法读取 .pck 文件名".to_string())?;
            let dll_name = dll_source
                .file_name()
                .ok_or_else(|| "无法读取 .dll 文件名".to_string())?;
            let game_mods = Path::new(&game_mods_path);
            let pck_target = game_mods.join(pck_name);
            let dll_target = game_mods.join(dll_name);
            let is_enabled = pck_target.exists() && dll_target.exists();
            mods.push(ModInfo { id: mod_id, enabled: is_enabled });
        }
    }
    Ok(mods)
}

#[tauri::command]
fn add_mod_from_files(
    pck_path: String,
    dll_path: String,
    library_path: String,
    mod_name: Option<String>
) -> Result<String, String> {
    let pck_source = Path::new(&pck_path);
    let dll_source = Path::new(&dll_path);

    if !pck_source.exists() || !pck_source.is_file() {
        return Err(".pck 文件路径无效".into());
    }
    if !dll_source.exists() || !dll_source.is_file() {
        return Err(".dll 文件路径无效".into());
    }

    let pck_ext = pck_source.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let dll_ext = dll_source.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    if pck_ext != "pck" {
        return Err("第一个文件必须是 .pck".into());
    }
    if dll_ext != "dll" {
        return Err("第二个文件必须是 .dll".into());
    }

    let resolved_name = match mod_name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => pck_source
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "无法从 .pck 文件名推断 Mod 名称".to_string())?
            .to_string()
    };

    let library = Path::new(&library_path);
    if !library.exists() {
        fs::create_dir_all(library).map_err(|e| e.to_string())?;
    }

    let target_mod_dir = library.join(&resolved_name);
    if target_mod_dir.exists() {
        return Err(format!("仓库中已存在同名 Mod: {}", resolved_name));
    }
    fs::create_dir_all(&target_mod_dir).map_err(|e| e.to_string())?;

    let pck_target = target_mod_dir.join(
        pck_source
            .file_name()
            .ok_or_else(|| "无法读取 .pck 文件名".to_string())?
    );
    let dll_target = target_mod_dir.join(
        dll_source
            .file_name()
            .ok_or_else(|| "无法读取 .dll 文件名".to_string())?
    );

    fs::copy(pck_source, pck_target).map_err(|e| e.to_string())?;
    fs::copy(dll_source, dll_target).map_err(|e| e.to_string())?;

    Ok(resolved_name)
}

#[tauri::command]
fn add_mod_from_folder(
    source_path: String,
    library_path: String,
    mod_name: Option<String>
) -> Result<String, String> {
    let source = Path::new(&source_path);
    if !source.exists() || !source.is_dir() {
        return Err("Mod 文件夹路径无效".into());
    }

    // 校验目录中至少有 .pck + .dll，避免导入后无法被识别。
    find_mod_pair_files(source)?;

    let resolved_name = match mod_name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "无法从文件夹名推断 Mod 名称".to_string())?
            .to_string()
    };

    let library = Path::new(&library_path);
    if !library.exists() {
        fs::create_dir_all(library).map_err(|e| e.to_string())?;
    }

    let target_mod_dir = library.join(&resolved_name);
    if target_mod_dir.exists() {
        return Err(format!("仓库中已存在同名 Mod: {}", resolved_name));
    }

    copy_dir_recursive(source, &target_mod_dir)?;
    Ok(resolved_name)
}

#[tauri::command]
fn toggle_mod(
    mod_id: String,
    library_path: String,
    game_mods_path: String,
    enable: bool
) -> Result<(), String> {
    let mod_dir = Path::new(&library_path).join(&mod_id);
    let source_files = list_mod_root_files(&mod_dir)?;
    let game_mods = Path::new(&game_mods_path);

    if !Path::new(&game_mods_path).exists() {
        fs::create_dir_all(&game_mods_path).map_err(|e| e.to_string())?;
    }

    if enable {
        for source in &source_files {
            let file_name = source
                .file_name()
                .ok_or_else(|| "无法读取 Mod 文件名".to_string())?;
            let target = game_mods.join(file_name);
            ensure_link_or_empty(&target, source)?;
        }

        for source in &source_files {
            let file_name = source
                .file_name()
                .ok_or_else(|| "无法读取 Mod 文件名".to_string())?;
            let target = game_mods.join(file_name);
            if target.exists() {
                continue;
            }
            #[cfg(target_os = "windows")]
            std::os::windows::fs::symlink_file(source, &target).map_err(|e| e.to_string())?;
            #[cfg(not(target_os = "windows"))]
            std::os::unix::fs::symlink(source, &target).map_err(|e| e.to_string())?;
        }
    } else {
        for source in &source_files {
            let file_name = source
                .file_name()
                .ok_or_else(|| "无法读取 Mod 文件名".to_string())?;
            let target = game_mods.join(file_name);
            remove_link_if_owned(&target, source)?;
        }
    }

    Ok(())
}
fn main() {
    tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![get_mods, toggle_mod, add_mod_from_files, add_mod_from_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}