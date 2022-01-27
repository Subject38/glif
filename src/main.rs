//! MFEKglif - A cross-platform .glif renderer and editor.
//! (c) 2020–2021 Fredrick R. Brennan, Matthew Blanchard & MFEK Authors
//! Apache 2.0 licensed. See AUTHORS.
#![allow(non_snake_case)] // for our name MFEKglif

use crate::command::{Command, CommandInfo, CommandMod};
use crate::editor::{
    events::{EditorEvent, IOEventType, MouseEventType},
    Editor,
};
use crate::tools::zoom::{zoom_in_factor, zoom_out_factor};
use crate::tools::ToolEnum;
use crate::user_interface::mouse_input::MouseInfo;
use crate::user_interface::{ImguiManager, Interface};

use ctrlc;
use enum_unitary::IntoEnumIterator;
use glifrenderer::toggles::{PointLabels, PreviewMode};
use sdl2::event::{Event, WindowEvent};
pub use skulpin::{rafx::api as RafxApi, skia_safe};

#[macro_use]
extern crate pub_mod;

use std::cell::RefCell;

pub mod args;
mod command;
pub mod constants;
mod contour_operations;
pub mod editor;
mod filedialog;
mod ipc;
mod render;
pub mod settings;
mod system_fonts;
mod tool_behaviors;
mod tools;
mod user_interface;
pub mod util;

fn main() {
    ipc::header();
    util::init_env_logger();
    util::set_panic_hook();

    let args = args::parse_args();
    let filename = args.filename.clone();
    let mut editor = Editor::new(args);

    let filename = filedialog::filename_or_panic(&filename, Some("glif"), None);
    let mut interface = Interface::new(filename.to_str().unwrap());
    let mut imgui_manager = ImguiManager::new(&mut interface);

    let mut skulpin_renderer = interface.initialize_skulpin_renderer(&interface.sdl_window);

    // Makes glyph available to on_load_glif events
    editor.load_glif(&mut interface, &filename);

    ctrlc::set_handler(util::quit_next_frame).expect("Could not set SIGTERM handler.");

    ipc::launch_fs_watcher(&mut editor);

    command::initialize_keybinds();
    // TODO: Replace console! tools::console::initialize_console_commands();

    let mut event_pump = interface.get_event_pump();
    'main_loop: loop {
        // Quit from console
        if editor.quit_requested {
            break 'main_loop;
        }

        let keys_down = interface.get_pressed_keys(&event_pump);
        let keymod = CommandMod::from_keys_down(&keys_down);

        // sdl event handling
        for event in event_pump.poll_iter() {
            util::log_sdl_event(&event);

            if let Event::Quit { .. } = &event {
                editor.quit(&mut interface);
            }

            if imgui_manager.handle_imgui_event(&event) {
                continue;
            }
            if interface.active_prompts() {
                continue;
            }

            // we're gonna handle console text input here as this should steal input from the command system
            /* TODO: Replace console!
            match &event {
                Event::TextInput { text, .. } => {
                    if CONSOLE.with(|c| return c.borrow_mut().active) {
                        for ch in text.chars() {
                            CONSOLE.with(|c| c.borrow_mut().handle_ch(ch));
                        }
                        continue;
                    }
                }
                _ => {}
            }
            */

            match event {
                Event::KeyDown { keycode, .. } => {
                    // we don't care about keydown events that have no keycode
                    if keycode.is_none() {
                        continue;
                    }
                    let keycode = keycode.unwrap();

                    /* TODO: Replace console!
                    tools::console::set_state(&mut editor, &mut interface, keycode, keymod);
                    if CONSOLE.with(|c| c.borrow_mut().active) {
                        continue;
                    } */

                    // check if we've got a command
                    let command_info: CommandInfo =
                        match command::keycode_to_command(&keycode, &keys_down) {
                            Some(command) => command,
                            None => continue,
                        };

                    let delete_after = RefCell::new(false);
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::ToolCommand {
                            command: command_info.command,
                            command_mod: command_info.command_mod,
                            stop_after: delete_after.clone(),
                        },
                    );
                    if *delete_after.borrow() {
                        continue;
                    }

                    log::trace!("Received command: {:?}", command_info.command);

                    match command_info.command {
                        Command::ResetScale => {
                            interface.update_viewport(None, Some(1.));
                        }
                        Command::ZoomIn => {
                            let scale = zoom_in_factor(&mut interface);
                            interface.update_viewport(None, Some(scale));
                        }
                        Command::ZoomOut => {
                            let scale = zoom_out_factor(&mut interface);
                            interface.update_viewport(None, Some(scale));
                        }
                        Command::ToolPan => {
                            editor.set_tool(ToolEnum::Pan);
                        }
                        Command::ToolPen => {
                            editor.set_tool(ToolEnum::Pen);
                        }
                        Command::ToolSelect => {
                            editor.set_tool(ToolEnum::Select);
                        }
                        Command::ToolZoom => {
                            editor.set_tool(ToolEnum::Zoom);
                        }
                        Command::ToolDash => {
                            editor.set_tool(ToolEnum::Dash);
                        }
                        Command::ToolPAP => {
                            editor.set_tool(ToolEnum::PAP);
                        }
                        Command::ToolVWS => {
                            editor.set_tool(ToolEnum::VWS);
                        }
                        Command::ToolMeasure => {
                            editor.set_tool(ToolEnum::Measure);
                        }
                        Command::ToolAnchors => {
                            editor.set_tool(ToolEnum::Anchors);
                        }
                        Command::ToolShapes => {
                            editor.set_tool(ToolEnum::Shapes);
                        }
                        Command::ToolGuidelines => {
                            editor.set_tool(ToolEnum::Guidelines);
                        }
                        Command::ToolGrid => {
                            editor.set_tool(ToolEnum::Grid);
                        }
                        Command::ToolImages => {
                            editor.set_tool(ToolEnum::Image);
                        }
                        Command::TogglePointLabels => {
                            trigger_toggle_on!(
                                interface,
                                point_labels,
                                PointLabels,
                                !command_info.command_mod.shift
                            );
                        }
                        Command::TogglePreviewMode => {
                            trigger_toggle_on!(
                                interface,
                                preview_mode,
                                PreviewMode,
                                !command_info.command_mod.shift
                            );
                        }
                        /* TODO: Replace console!
                        Command::ToggleConsole => {
                            CONSOLE.with(|c| {
                                c.borrow_mut().active = true;
                            });
                        }*/
                        Command::DeleteSelection => {
                            editor.delete_selection();
                        }
                        Command::SelectAll => {} // handled by select tool, only when select active
                        Command::CopySelection => {
                            editor.copy_selection();
                        }
                        Command::PasteSelection => {
                            editor.paste_selection(Some(interface.mouse_info.position));
                        }
                        Command::PasteSelectionInPlace => {
                            editor.paste_selection(None);
                        }
                        Command::CutSelection => {
                            editor.copy_selection();
                            editor.delete_selection();
                        }
                        Command::HistoryUndo => {
                            editor.undo();
                        }
                        Command::HistoryRedo => {
                            editor.redo();
                        }
                        Command::IOOpen => {
                            let filename =
                                match filedialog::open_filename(Some("glif,glifjson"), None) {
                                    Some(f) => f,
                                    None => continue,
                                };
                            editor.load_glif(&mut interface, &filename);
                        }
                        Command::IOSave => {
                            drop(editor.save_glif(false));
                            editor.dispatch_editor_event(
                                &mut interface,
                                EditorEvent::IOEvent {
                                    event_type: IOEventType::FileSaved,
                                    path: filename.clone(),
                                },
                            );
                        }
                        Command::IOSaveAs => match editor.save_glif(true) {
                            Ok(pb) => {
                                editor.dispatch_editor_event(
                                    &mut interface,
                                    EditorEvent::IOEvent {
                                        event_type: IOEventType::FileSavedAs,
                                        path: pb.clone(),
                                    },
                                );
                                editor.load_glif(&mut interface, &pb);
                            }
                            Err(()) => {}
                        },
                        Command::IOFlatten | Command::IOFlattenAs => {
                            let rename = command_info.command == Command::IOFlattenAs;
                            let event_type = if rename {
                                IOEventType::FileFlattenedAs
                            } else {
                                IOEventType::FileFlattened
                            };
                            match editor.flatten_glif(Some(&mut interface), rename) {
                                Ok(filename) => editor.dispatch_editor_event(
                                    &mut interface,
                                    EditorEvent::IOEvent {
                                        event_type,
                                        path: filename,
                                    },
                                ),
                                Err(()) => {}
                            }
                        }
                        Command::IOExport => {
                            if let Ok(()) = editor.export_glif(Some(&mut interface)) {
                                editor.dispatch_editor_event(
                                    &mut interface,
                                    EditorEvent::IOEvent {
                                        event_type: IOEventType::FileExported,
                                        path: filename.clone(),
                                    },
                                );
                            }
                        }
                        Command::Quit => {
                            editor.quit(&mut interface);
                        }
                        // TODO: More elegantly deal with Command's meant for consumption by a
                        // single tool?
                        Command::ReverseContour => {
                            log::debug!("Tried to reverse contour outside Select tool");
                        }
                        Command::SkiaDump => {
                            editor.skia_dump();
                        }
                        #[allow(unreachable_patterns)]
                        // This failsafe is here if you add a Command.
                        cmd => log::error!("Command unimplemented: {:?}", cmd),
                    }
                }

                Event::MouseMotion { x, y, .. } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(&mut interface, None, position, None, keymod);
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::Moved,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseButtonDown {
                    mouse_btn,
                    x,
                    y,
                    clicks: 2,
                    ..
                } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(
                        &mut interface,
                        Some(mouse_btn),
                        position,
                        Some(true),
                        keymod,
                    );
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::DoubleClick,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(
                        &mut interface,
                        Some(mouse_btn),
                        position,
                        Some(true),
                        keymod,
                    );
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::Pressed,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(
                        &mut interface,
                        Some(mouse_btn),
                        position,
                        Some(false),
                        keymod,
                    );
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::Released,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseWheel { x, y, .. } => {
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::ScrollEvent {
                            horizontal: x,
                            vertical: y,
                        },
                    );
                }

                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::SizeChanged(x, y) | WindowEvent::Resized(x, y) => {
                        interface.viewport.winsize = (x as f32, y as f32);
                        interface.viewport.set_broken_flag();
                        interface.adjust_viewport_by_os_dpi();
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        editor.rebuild(&mut interface);
        interface.render(
            &mut editor,
            &mut imgui_manager.imgui_context,
            &mut imgui_manager.imgui_sdl2,
            &mut imgui_manager.imgui_renderer,
            &mut skulpin_renderer,
            &event_pump.mouse_state(),
        );
    }
}
