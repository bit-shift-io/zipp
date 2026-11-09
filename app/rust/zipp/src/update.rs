use std::time::Duration;

use iced::{Subscription, Task};

use crate::app::App;
use crate::message::Message;

impl App {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop | Message::WindowOpened(_) => Task::none(),
            Message::Tick => {
                self.now = zipp_core::log::now();
                Task::none()
            }
            Message::Transport(_event) => Task::none(),
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick)])
    }
}
