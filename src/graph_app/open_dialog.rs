use std::path::PathBuf;

use relm4_components::open_dialog::{OpenDialogParent, OpenDialogSettings};

use super::{AppModel, AppMsg};

pub struct OpenDialogConfig {}

// Настройки диалога открытия файла
impl relm4_components::open_dialog::OpenDialogConfig for OpenDialogConfig {
    type Model = AppModel;

    fn open_dialog_config(_model: &Self::Model) -> OpenDialogSettings {
        OpenDialogSettings {
            accept_label: "Открыть",
            cancel_label: "Отмена",
            create_folders: true,
            is_modal: true,
            filters: Vec::new(),
        }
    }
}

impl OpenDialogParent for AppModel {
    fn open_msg(path: PathBuf) -> Self::Msg {
        AppMsg::OpenFile(path)
    }
}
