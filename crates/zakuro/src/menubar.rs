//! the menu bar, macOS's at the top of the screen and the one in the window
//! on Windows, with what the menu Escape opens over a game has, the choices
//! of the settings that are lists, and the cheats. elsewhere there is none,
//! and the menu over the game is the only one.

use crate::settings::{Filter, Screens, Stereo3d, TopShape};

/// what a click on the bar asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Resume,
    Pause,
    Reset,
    SaveState,
    LoadState,
    Slot(u8),
    Fullscreen,
    Library,
    ChooseFolder,
    CopyReport,
    Quit,
    Settings,
    Cheats,
    /// a cheat by its place in the game's list, turned the other way.
    Cheat(usize),
    ReloadCheats,
    CheatsFolder,
    Multiplayer,
    CameraPicture,
    /// how many times the console's resolution the GPU draws the 3D at.
    Resolution(u32),
    Screens(Screens),
    TopShape(TopShape),
    Filter(Filter),
    Stereo(Stereo3d),
    IntegerScale,
    ShowFps,
    Mute,
}

/// what the bar shows ticked and greyed out, from the app's state.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown {
    /// whether a game runs, the bar's game items only work then.
    pub game: bool,
    pub paused: bool,
    pub slot: u8,
    /// whether the GPU draws the 3D, the resolution only counts then.
    pub on_gpu: bool,
    pub resolution: u32,
    pub screens: Screens,
    pub top_shape: TopShape,
    pub filter: Filter,
    pub stereo: Stereo3d,
    pub integer_scale: bool,
    pub show_fps: bool,
    pub mute: bool,
    /// the running game's cheats, by name, and whether each is on.
    pub cheats: Vec<(String, bool)>,
}

/// the state slots, as the menu over the game counts them.
const SLOTS: std::ops::RangeInclusive<u8> = 1..=9;
/// the resolutions, as the settings window has them.
const RESOLUTIONS: std::ops::RangeInclusive<u32> = 1..=8;

#[cfg(any(target_os = "macos", windows))]
pub use native::MenuBar;

#[cfg(not(any(target_os = "macos", windows)))]
pub use none::MenuBar;

#[cfg(any(target_os = "macos", windows))]
mod native {
    use std::collections::HashMap;

    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use winit::window::Window;

    use super::{Command, Shown, RESOLUTIONS, SLOTS};
    use crate::settings::{Filter, Screens, Stereo3d, TopShape};

    /// a shortcut on macOS, where the menus take the keys with Command
    /// before the window sees them. Windows menus get no shortcuts, which
    /// would need the event loop to hand them its messages, the game's keys
    /// and F keys work the same there.
    fn key(mods: Modifiers, code: Code) -> Option<Accelerator> {
        cfg!(target_os = "macos").then(|| Accelerator::new(mods, code))
    }

    fn cmd(code: Code) -> Option<Accelerator> {
        key(Modifiers::META, code)
    }

    /// a list of choices, ticked one at a time.
    struct Choices<T> {
        items: Vec<(T, CheckMenuItem)>,
    }

    impl<T: Copy + PartialEq> Choices<T> {
        fn show(&self, chosen: T) {
            for (value, item) in &self.items {
                item.set_checked(*value == chosen);
            }
        }
    }

    pub struct MenuBar {
        menu: Menu,
        commands: HashMap<String, Command>,
        /// the items that only work with a game running.
        for_game: Vec<Box<dyn Fn(bool)>>,
        choose_folder: MenuItem,
        pause: CheckMenuItem,
        slots: Choices<u8>,
        resolutions: Choices<u32>,
        resolution_menu: Submenu,
        screens: Choices<Screens>,
        top_shapes: Choices<TopShape>,
        filters: Choices<Filter>,
        stereo: Choices<Stereo3d>,
        integer_scale: CheckMenuItem,
        show_fps: CheckMenuItem,
        mute: CheckMenuItem,
        cheats: Submenu,
        /// how many items come before the game's cheats in their menu.
        cheats_after: usize,
        cheat_items: Vec<CheckMenuItem>,
        shown: Option<Shown>,
    }

    impl MenuBar {
        /// the bar, put up for the app on macOS and in the window on
        /// Windows, none when it can't be.
        pub fn new(window: &Window) -> Option<MenuBar> {
            let mut bar = MenuBar::build().inspect_err(|error| log::warn!("no menu bar, {error}")).ok()?;
            bar.attach(window)?;
            Some(bar)
        }

        #[cfg(target_os = "macos")]
        fn attach(&mut self, _window: &Window) -> Option<()> {
            self.menu.init_for_nsapp();
            Some(())
        }

        #[cfg(windows)]
        fn attach(&mut self, window: &Window) -> Option<()> {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else { return None };
            // SAFETY: the window's own handle, which outlives the bar, the
            // app dropping the bar before the window
            unsafe { self.menu.init_for_hwnd(handle.hwnd.get()) }.inspect_err(|error| log::warn!("no menu bar, {error}")).ok()
        }

        fn build() -> muda::Result<MenuBar> {
            let menu = Menu::new();
            let mut commands = HashMap::new();
            let mut for_game: Vec<Box<dyn Fn(bool)>> = Vec::new();
            // an item of its own id, which the click comes back with
            let mut item = |text: &str, command: Command, accelerator: Option<Accelerator>| {
                let id = format!("{command:?}");
                commands.insert(id.clone(), command);
                MenuItem::with_id(id, text, true, accelerator)
            };
            let mac = cfg!(target_os = "macos");

            let settings = item("Settings…", Command::Settings, cmd(Code::Comma));
            let quit = item(if mac { "Quit Zakuro" } else { "Quit" }, Command::Quit, cmd(Code::KeyQ));
            if mac {
                let app = Submenu::new("Zakuro", true);
                let about = muda::AboutMetadata { name: Some("Zakuro".into()), version: Some(env!("CARGO_PKG_VERSION").into()), ..Default::default() };
                app.append_items(&[
                    &PredefinedMenuItem::about(Some("About Zakuro"), Some(about)),
                    &PredefinedMenuItem::separator(),
                    &settings,
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::services(None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::hide(Some("Hide Zakuro")),
                    &PredefinedMenuItem::hide_others(None),
                    &PredefinedMenuItem::show_all(None),
                    &PredefinedMenuItem::separator(),
                    &quit,
                ])?;
                menu.append(&app)?;
            }

            let resume = item("Resume", Command::Resume, None);
            let reset = item("Reset", Command::Reset, cmd(Code::KeyR));
            let save = item("Save state (F5)", Command::SaveState, cmd(Code::KeyS));
            let load = item("Load state (F8)", Command::LoadState, cmd(Code::KeyL));
            let pause = CheckMenuItem::with_id(format!("{:?}", Command::Pause), "Pause (F1)", true, false, cmd(Code::KeyP));
            commands.insert(format!("{:?}", Command::Pause), Command::Pause);
            let mut check = |text: &str, command: Command, accelerator: Option<Accelerator>| {
                let id = format!("{command:?}");
                commands.insert(id.clone(), command);
                CheckMenuItem::with_id(id, text, true, false, accelerator)
            };
            let slots = Choices { items: SLOTS.map(|slot| (slot, check(&format!("Slot {slot}"), Command::Slot(slot), None))).collect() };
            let slot_menu = Submenu::new("State slot", true);
            for (_, item) in &slots.items {
                slot_menu.append(item)?;
            }
            let mut item = |text: &str, command: Command, accelerator: Option<Accelerator>| {
                let id = format!("{command:?}");
                commands.insert(id.clone(), command);
                MenuItem::with_id(id, text, true, accelerator)
            };
            let library = item("Back to the library", Command::Library, None);
            let choose_folder = item("Choose the games folder…", Command::ChooseFolder, None);
            let fullscreen = item("Fullscreen (F11)", Command::Fullscreen, key(Modifiers::META | Modifiers::CONTROL, Code::KeyF));
            let camera = item("Show the camera a picture…", Command::CameraPicture, None);
            let multiplayer = item("Multiplayer…", Command::Multiplayer, None);
            let report = item("Copy info for a report", Command::CopyReport, None);
            let cheats_window = item("Cheats…", Command::Cheats, None);
            let reload = item("Reload the cheats", Command::ReloadCheats, None);
            let cheats_folder = item("Open the cheats folder", Command::CheatsFolder, None);

            let game = Submenu::new("&Game", true);
            game.append_items(&[
                &resume,
                &pause,
                &reset,
                &PredefinedMenuItem::separator(),
                &save,
                &load,
                &slot_menu,
                &PredefinedMenuItem::separator(),
                &library,
                &choose_folder,
            ])?;
            if !mac {
                game.append_items(&[&PredefinedMenuItem::separator(), &quit])?;
            }
            menu.append(&game)?;

            let mut check = |text: &str, command: Command, accelerator: Option<Accelerator>| {
                let id = format!("{command:?}");
                commands.insert(id.clone(), command);
                CheckMenuItem::with_id(id, text, true, false, accelerator)
            };
            let resolutions = Choices {
                items: RESOLUTIONS.map(|scale| (scale, check(&format!("{scale}x"), Command::Resolution(scale), None))).collect(),
            };
            let digits = [Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4, Code::Digit5];
            let screens = Choices {
                items: Screens::ALL.iter().zip(digits).map(|(&screens, digit)| (screens, check(screens.name(), Command::Screens(screens), cmd(digit)))).collect(),
            };
            let top_shapes = Choices { items: TopShape::ALL.map(|shape| (shape, check(shape.name(), Command::TopShape(shape), None))).into() };
            let filters = Choices { items: Filter::ALL.map(|filter| (filter, check(filter.name(), Command::Filter(filter), None))).into() };
            let stereo = Choices { items: Stereo3d::ALL.map(|stereo| (stereo, check(stereo.name(), Command::Stereo(stereo), None))).into() };
            let integer_scale = check("Scale by whole numbers only", Command::IntegerScale, None);
            let show_fps = check("Show the frame rate", Command::ShowFps, None);
            let mute = check("Mute", Command::Mute, None);
            let list = |text: &str, items: Vec<&CheckMenuItem>| -> muda::Result<Submenu> {
                let submenu = Submenu::new(text, true);
                for item in items {
                    submenu.append(item)?;
                }
                Ok(submenu)
            };
            let resolution_menu = list("Resolução Interna", resolutions.items.iter().map(|(_, item)| item).collect())?;
            let view = Submenu::new("&View", true);
            view.append_items(&[
                &resolution_menu,
                &PredefinedMenuItem::separator(),
                &list("Screens (F9)", screens.items.iter().map(|(_, item)| item).collect())?,
                &list("Top screen shape", top_shapes.items.iter().map(|(_, item)| item).collect())?,
                &list("Filter", filters.items.iter().map(|(_, item)| item).collect())?,
                &integer_scale,
                &list("3D", stereo.items.iter().map(|(_, item)| item).collect())?,
                &PredefinedMenuItem::separator(),
                &show_fps,
                &fullscreen,
            ])?;
            menu.append(&view)?;

            let cheats = Submenu::new("&Cheats", true);
            let fixed: [&dyn IsMenuItem; 4] = [&cheats_window, &reload, &cheats_folder, &PredefinedMenuItem::separator()];
            cheats.append_items(&fixed)?;
            let cheats_after = fixed.len();
            menu.append(&cheats)?;

            let tools = Submenu::new("&Tools", true);
            tools.append_items(&[&multiplayer, &camera, &PredefinedMenuItem::separator(), &mute])?;
            if !mac {
                tools.append_items(&[&PredefinedMenuItem::separator(), &settings])?;
            }
            menu.append(&tools)?;

            if mac {
                let window = Submenu::new("Window", true);
                window.append_items(&[
                    &PredefinedMenuItem::minimize(None),
                    &PredefinedMenuItem::maximize(Some("Zoom")),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::bring_all_to_front(None),
                ])?;
                menu.append(&window)?;
            }
            let help = Submenu::new("&Help", true);
            help.append(&report)?;
            menu.append(&help)?;

            for item in [resume, reset, save, load, library, camera, report, reload, cheats_folder, cheats_window] {
                for_game.push(Box::new(move |on| item.set_enabled(on)));
            }
            for_game.push(Box::new(move |on| slot_menu.set_enabled(on)));
            let paused = pause.clone();
            for_game.push(Box::new(move |on| paused.set_enabled(on)));

            Ok(MenuBar {
                menu,
                commands,
                for_game,
                choose_folder,
                pause,
                slots,
                resolutions,
                resolution_menu,
                screens,
                top_shapes,
                filters,
                stereo,
                integer_scale,
                show_fps,
                mute,
                cheats,
                cheats_after,
                cheat_items: Vec::new(),
                shown: None,
            })
        }

        /// what was clicked since the last time.
        pub fn commands(&mut self) -> Vec<Command> {
            let mut commands = Vec::new();
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                commands.extend(self.commands.get(event.id().as_ref()).copied());
            }
            // a check item ticks itself when clicked, the next show puts
            // what the app made of it back
            if !commands.is_empty() {
                self.shown = None;
            }
            commands
        }

        /// ticks and greys out the items as the app's state says, when it
        /// changed.
        pub fn show(&mut self, shown: &Shown) {
            let last = self.shown.as_ref();
            if last == Some(shown) {
                return;
            }
            if last.is_none_or(|last| last.game != shown.game) {
                for set in &self.for_game {
                    set(shown.game);
                }
                self.choose_folder.set_enabled(!shown.game);
            }
            if last.is_none_or(|last| last.cheats.iter().map(|(name, _)| name).ne(shown.cheats.iter().map(|(name, _)| name))) {
                self.list_cheats(&shown.cheats);
            }
            self.pause.set_checked(shown.paused);
            self.slots.show(shown.slot);
            self.resolutions.show(shown.resolution);
            self.resolution_menu.set_enabled(shown.on_gpu);
            self.screens.show(shown.screens);
            self.top_shapes.show(shown.top_shape);
            self.filters.show(shown.filter);
            self.stereo.show(shown.stereo);
            self.integer_scale.set_checked(shown.integer_scale);
            self.show_fps.set_checked(shown.show_fps);
            self.mute.set_checked(shown.mute);
            for (item, (_, on)) in self.cheat_items.iter().zip(&shown.cheats) {
                item.set_checked(*on);
            }
            self.shown = Some(shown.clone());
        }

        /// the game's cheats in their menu, after its own items, in place of
        /// the ones listed before.
        fn list_cheats(&mut self, cheats: &[(String, bool)]) {
            for item in self.cheat_items.drain(..) {
                let _ = self.cheats.remove(&item);
                self.commands.remove(item.id().as_ref());
            }
            for (index, (name, on)) in cheats.iter().enumerate() {
                let command = Command::Cheat(index);
                let id = format!("{command:?}");
                // a & in a name would underline the letter after it on Windows
                let item = CheckMenuItem::with_id(id.clone(), name.replace('&', "&&"), true, *on, None);
                if self.cheats.insert(&item, self.cheats_after + index).is_ok() {
                    self.commands.insert(id, command);
                    self.cheat_items.push(item);
                }
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod none {
    use winit::window::Window;

    use super::{Command, Shown};

    /// no bar, on a system whose windows have none the app can fill.
    pub struct MenuBar;

    impl MenuBar {
        pub fn new(_window: &Window) -> Option<MenuBar> {
            None
        }

        pub fn commands(&mut self) -> Vec<Command> {
            Vec::new()
        }

        pub fn show(&mut self, _shown: &Shown) {}
    }
}
