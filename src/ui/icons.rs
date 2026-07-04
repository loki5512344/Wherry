//! Solar icon set (https://solar-icons.vercel.app) — встроенные SVG, красятся через tint()
use egui::{Color32, Ui};

macro_rules! icon_set {
    ($($key:ident => $file:literal),* $(,)?) => {
        pub enum Icon { $($key),* }

        fn bytes(icon: &Icon) -> &'static [u8] {
            match icon {
                $(Icon::$key => include_bytes!(concat!("icons/svg/", $file, ".svg"))),*
            }
        }

        fn uri(icon: &Icon) -> &'static str {
            match icon {
                $(Icon::$key => concat!("bytes://solar/", $file, ".svg")),*
            }
        }
    };
}

icon_set! {
    Folder => "folder-bold",
    FolderWithFiles => "folder-with-files-linear",
    AddCircleLinear => "add-circle-linear",
    AddCircleBold => "add-circle-bold",
    AddSquare => "add-square-linear",
    Upload => "upload-linear",
    Download => "download-linear",
    DownloadSquare => "download-square-bold",
    Clock => "clock-circle-linear",
    Refresh => "refresh-linear",
    Trash => "trash-bin-minimalistic-linear",
    Pen => "pen-linear",
    History => "history-linear",
    ServerSquare => "server-square-linear",
    Home => "home-2-bold",
    Monitor => "monitor-bold",
    Document => "document-bold",
    DocumentText => "document-text-bold",
    Ssd => "ssd-square-linear",
    Database => "database-linear",
    DangerTriangle => "danger-triangle-bold",
    LockPassword => "lock-password-bold",
    ArrowDown => "alt-arrow-down-linear",
    ArrowUp => "alt-arrow-up-linear",
    CheckCircle => "check-circle-bold",
    PlayCircle => "play-circle-linear",
    PauseCircle => "pause-circle-linear",
    FileText => "file-text-bold",
    Gallery => "gallery-bold",
    Code => "code-bold",
    Archive => "archive-down-minimlistic-bold",
    ShieldKeyhole => "shield-keyhole-bold",
    File => "file-bold",
    Star => "star-bold",
    Videocamera => "videocamera-record-bold",
    MusicNote => "music-note-3-bold",
    Link => "link-bold",
    CloseCircle => "close-circle-linear",
    Settings => "settings-linear",
}

/// Рисует иконку заданного размера и цвета (tint) в текущей позиции ui.
pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> egui::Response {
    let image = egui::Image::from_bytes(uri(&icon), bytes(&icon))
        .tint(color)
        .fit_to_exact_size(egui::vec2(size, size));
    ui.add(image)
}

/// Тот же рендер, но для использования внутри builder-паттернов (кнопки и т.д.) — возвращает Image.
pub fn image(icon: Icon, size: f32, color: Color32) -> egui::Image<'static> {
    egui::Image::from_bytes(uri(&icon), bytes(&icon))
        .tint(color)
        .fit_to_exact_size(egui::vec2(size, size))
}

/// Цвет + иконка по расширению файла — палитра Clay
pub fn file_icon_for(ext: &str) -> (Icon, Color32) {
    use crate::ui::theme::*;
    match ext {
        "pdf" => (Icon::FileText, RED),
        "xlsx" | "xls" | "doc" | "docx" => (Icon::Document, GREEN),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => (Icon::Gallery, ACCENT),
        "mp4" | "mkv" | "avi" | "mov" => (Icon::Videocamera, ACCENT),
        "mp3" | "ogg" | "flac" | "wav" => (Icon::MusicNote, GREEN),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => (Icon::Archive, TEXT_DIM),
        "yaml" | "yml" | "toml" | "json" => (Icon::Code, YELLOW),
        "php" | "java" | "class" | "jar" => (Icon::Code, ACCENT_DIM),
        "rs" | "py" | "sh" | "bash" | "fish" | "zsh" => (Icon::Code, GREEN),
        "html" | "css" | "js" | "ts" => (Icon::Code, ACCENT),
        "log" => (Icon::DocumentText, TEXT_HINT),
        "htaccess" => (Icon::ShieldKeyhole, TEXT_DIM),
        "txt" | "md" => (Icon::DocumentText, TEXT_DIM),
        _ => (Icon::File, TEXT_DIM),
    }
}
