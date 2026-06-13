use tauri::State;
use crate::domain::site::Site;
use crate::storage::db::Db;

#[tauri::command]
pub fn get_sites(db: State<'_, Db>) -> Result<Vec<Site>, String> {
    let conn = db.0.lock().unwrap();
    crate::storage::db::get_sites(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_site(db: State<'_, Db>, site: Site) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    crate::storage::db::save_site(&conn, &site).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_site(db: State<'_, Db>, id: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    crate::storage::db::delete_site(&conn, &id).map_err(|e| e.to_string())
}
