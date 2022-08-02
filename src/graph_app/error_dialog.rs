use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::{send, ComponentUpdate, Model, Sender, Widgets};

use super::{AppModel, AppMsg};

// Модель данных для сообщения об ошибке
pub struct ErrorDialogModel {
    hidden: bool,          // скрыт ли диалог
    error: Option<String>, // текст ошибки
}

// Сообщения к модели данных
pub enum ErrorDialogMsg {
    Show(String), // показать диалог с заданной ошибкой
    Accept,       // закрыть диалог
}

impl Model for ErrorDialogModel {
    type Msg = ErrorDialogMsg;
    type Widgets = ErrorDialogWidgets;
    type Components = ();
}

impl ComponentUpdate<AppModel> for ErrorDialogModel {
    // Инициализация модели
    fn init_model(_parent_model: &AppModel) -> Self {
        ErrorDialogModel {
            hidden: true,
            error: None,
        }
    }

    // Обработка сообщений к модели
    fn update(
        &mut self,
        msg: ErrorDialogMsg,
        _components: &(),
        _sender: Sender<ErrorDialogMsg>,
        _parent_sender: Sender<AppMsg>,
    ) {
        match msg {
            ErrorDialogMsg::Show(error) => {
                self.error = Some(error);
                self.hidden = false;
            }
            ErrorDialogMsg::Accept => self.hidden = true,
        }
    }
}

// Интерфейс диалога об ошибке
#[relm4_macros::widget(pub)]
impl Widgets<ErrorDialogModel, AppModel> for ErrorDialogWidgets {
    view! {
        gtk::MessageDialog {
            set_transient_for: parent!(Some(&parent_widgets.main_window)),
            set_modal: true,
            set_visible: watch!(!model.hidden),
            set_text: Some("Ошибка"),
            set_secondary_text: watch!(model.error.as_deref()),
            set_message_type: gtk::MessageType::Error,
            add_button: args!("ОК", gtk::ResponseType::Accept),
            connect_response(sender) => move |_, _| {
                send!(sender, ErrorDialogMsg::Accept);
            }
        }
    }
}
