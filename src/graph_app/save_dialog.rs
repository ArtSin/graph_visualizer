use std::path::PathBuf;

use relm4_components::save_dialog::{SaveDialogParent, SaveDialogSettings};

use super::{AppModel, AppMsg};

pub struct SaveDialogConfig {}

// Настройки диалога сохранения файла
impl relm4_components::save_dialog::SaveDialogConfig for SaveDialogConfig {
    type Model = AppModel;

    fn dialog_config(_model: &Self::Model) -> SaveDialogSettings {
        SaveDialogSettings {
            accept_label: "Сохранить",
            cancel_label: "Отмена",
            create_folders: true,
            is_modal: true,
            filters: Vec::new(),
        }
    }
}

impl SaveDialogParent for AppModel {
    fn save_msg(path: PathBuf) -> Self::Msg {
        AppMsg::SaveFile(path)
    }
}
