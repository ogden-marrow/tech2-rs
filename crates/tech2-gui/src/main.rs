#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use serde_json::{json, Value};

const APP_ID: &str = "com.github.ogdenmarrow.Tech2Rs";
const CSS: &str = "
.hero-title { font-size: 28px; font-weight: 800; }
.hero-subtitle { font-size: 14px; }
.warning { color: #e5a50a; }
.mono { font-family: monospace; }
";

struct Ui {
    window: adw::ApplicationWindow,
    port_dropdown: gtk::DropDown,
    ports: RefCell<Vec<String>>,
    image_label: gtk::Label,
    image_warning: gtk::Label,
    data_label: gtk::Label,
    status: gtk::Label,
    toast: adw::ToastOverlay,
    image_path: RefCell<Option<String>>,
    data_dir: RefCell<Option<String>>,
    socket: PathBuf,
}

fn main() -> glib::ExitCode {
    adw::init().expect("libadwaita init");
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    install_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("tech2-rs")
        .default_width(980)
        .default_height(780)
        .build();

    let header = adw::HeaderBar::new();
    let refresh = gtk::Button::with_label("Refresh");
    header.pack_end(&refresh);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(20)
        .margin_end(20)
        .build();

    let hero = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .build();
    let icon = gtk::Image::from_icon_name("drive-harddisk-symbolic");
    icon.set_pixel_size(64);
    hero.append(&icon);
    let hero_text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .valign(gtk::Align::Center)
        .build();
    let title = gtk::Label::builder()
        .label("Tech 2 toolchain")
        .xalign(0.0)
        .build();
    title.add_css_class("hero-title");
    let subtitle = gtk::Label::builder()
        .label("Native card images, read-only TIS data, and a local API for your Saab.")
        .xalign(0.0)
        .wrap(true)
        .build();
    subtitle.add_css_class("hero-subtitle");
    subtitle.add_css_class("dim-label");
    hero_text.append(&title);
    hero_text.append(&subtitle);
    hero.append(&hero_text);
    content.append(&hero);

    let connection = adw::PreferencesGroup::builder().title("Connection").build();
    let port_row = adw::ActionRow::builder()
        .title("Serial port")
        .subtitle("Select the USB-to-RS232 adapter connected to the Tech 2")
        .build();
    let port_dropdown = gtk::DropDown::from_strings(&["No serial ports found"]);
    port_dropdown.set_sensitive(false);
    let port_refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
    port_refresh.set_tooltip_text(Some("Scan for serial ports"));
    let port_suffix = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    port_suffix.append(&port_dropdown);
    port_suffix.append(&port_refresh);
    port_row.add_suffix(&port_suffix);
    connection.add(&port_row);
    content.append(&connection);

    let image_group = adw::PreferencesGroup::builder().title("Card image").build();
    let image_row = adw::ActionRow::builder()
        .title("Image")
        .subtitle("Choose a Tech 2 .bin image to validate")
        .build();
    let choose_image = gtk::Button::with_label("Choose…");
    image_row.add_suffix(&choose_image);
    image_group.add(&image_row);
    content.append(&image_group);

    let image_label = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label("No image selected")
        .build();
    image_label.add_css_class("dim-label");
    content.append(&image_label);
    let image_warning = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    image_warning.add_css_class("warning");
    content.append(&image_warning);

    let data_group = adw::PreferencesGroup::builder()
        .title("TIS2000 data")
        .build();
    let data_row = adw::ActionRow::builder()
        .title("Data folder")
        .subtitle("Read-only Access/SuperBase probes; originals are never modified")
        .build();
    let choose_data = gtk::Button::with_label("Choose…");
    data_row.add_suffix(&choose_data);
    data_group.add(&data_row);
    let probe_row = adw::ActionRow::builder()
        .title("Probe")
        .subtitle("List COSIDS.MDB tables and SuperBase headers")
        .build();
    let probe = gtk::Button::with_label("Probe");
    probe_row.add_suffix(&probe);
    data_group.add(&probe_row);
    content.append(&data_group);

    let data_label = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label("No data folder selected")
        .build();
    data_label.add_css_class("dim-label");
    content.append(&data_label);

    let card_group = adw::PreferencesGroup::builder()
        .title("Card writer")
        .build();
    let card_row = adw::ActionRow::builder()
        .title("Native writer")
        .subtitle("Waiting for the captured serial protocol")
        .build();
    let write = gtk::Button::with_label("Write card");
    write.set_sensitive(false);
    write.set_tooltip_text(Some("Native card-writer protocol is not implemented yet"));
    card_row.add_suffix(&write);
    card_group.add(&card_row);
    content.append(&card_group);

    let status = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label("Starting…")
        .build();
    status.add_css_class("dim-label");
    content.append(&status);

    let clamp = adw::Clamp::builder().maximum_size(900).build();
    clamp.set_child(Some(&content));
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).build();
    scroll.set_child(Some(&clamp));
    toolbar.set_content(Some(&scroll));

    let toast = adw::ToastOverlay::new();
    toast.set_child(Some(&toolbar));
    window.set_content(Some(&toast));

    let ui = Rc::new(Ui {
        window: window.clone(),
        port_dropdown: port_dropdown.clone(),
        ports: RefCell::new(Vec::new()),
        image_label,
        image_warning,
        data_label,
        status,
        toast,
        image_path: RefCell::new(None),
        data_dir: RefCell::new(None),
        socket: tech2d::default_socket_path(),
    });

    {
        let ui = Rc::clone(&ui);
        refresh.connect_clicked(move |_| refresh_ports(&ui));
    }
    {
        let ui = Rc::clone(&ui);
        port_refresh.connect_clicked(move |_| refresh_ports(&ui));
    }
    {
        let ui = Rc::clone(&ui);
        choose_image.connect_clicked(move |_| choose_image_dialog(&ui));
    }
    {
        let ui = Rc::clone(&ui);
        choose_data.connect_clicked(move |_| choose_data_dialog(&ui));
    }
    {
        let ui = Rc::clone(&ui);
        probe.connect_clicked(move |_| probe_data(&ui));
    }
    {
        let ui = Rc::clone(&ui);
        write.connect_clicked(move |_| write_card(&ui));
    }

    window.present();
    if let Err(err) = ensure_daemon(&ui.socket) {
        set_status(&ui, &format!("tech2d: {err}"));
    } else {
        refresh_ports(&ui);
        if let Ok(value) = rpc(&ui, "status", json!({})) {
            let version = value["version"].as_str().unwrap_or("?");
            set_status(&ui, &format!("Connected to tech2d {version}."));
        }
    }
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn ensure_daemon(socket: &Path) -> Result<(), String> {
    if tech2d::rpc_call(socket, "status", json!({})).is_ok() {
        return Ok(());
    }
    let binary = env::var_os("TECH2D_BIN")
        .map(PathBuf::from)
        .or_else(|| {
            let exe = env::current_exe().ok()?;
            let candidate = exe.parent()?.join("tech2d");
            candidate.is_file().then_some(candidate)
        })
        .unwrap_or_else(|| PathBuf::from("tech2d"));
    Command::new(binary)
        .arg("--socket")
        .arg(socket)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("could not start tech2d: {err}"))?;
    for _ in 0..30 {
        if tech2d::rpc_call(socket, "status", json!({})).is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(100));
    }
    Err("tech2d did not become ready".to_string())
}

fn rpc(ui: &Ui, method: &str, params: Value) -> Result<Value, String> {
    tech2d::rpc_call(&ui.socket, method, params).map_err(|err| err.to_string())
}

fn set_status(ui: &Ui, message: &str) {
    ui.status.set_label(message);
}

fn toast(ui: &Ui, message: &str) {
    ui.toast.add_toast(adw::Toast::new(message));
}

fn refresh_ports(ui: &Ui) {
    match rpc(ui, "ports.list", json!({})) {
        Ok(value) => {
            let names: Vec<String> = value["ports"]
                .as_array()
                .map(|ports| {
                    ports
                        .iter()
                        .filter_map(|port| port["device"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            *ui.ports.borrow_mut() = names.clone();
            if names.is_empty() {
                let model = gtk::StringList::new(&["No serial ports found"]);
                ui.port_dropdown.set_model(Some(&model));
                ui.port_dropdown.set_sensitive(false);
            } else {
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let model = gtk::StringList::new(&refs);
                ui.port_dropdown.set_model(Some(&model));
                ui.port_dropdown.set_sensitive(true);
                ui.port_dropdown.set_selected(0);
            }
        }
        Err(err) => set_status(ui, &format!("Port scan failed: {err}")),
    }
}

fn choose_image_dialog(ui: &Rc<Ui>) {
    let dialog = gtk::FileDialog::builder()
        .title("Choose a Tech 2 card image")
        .build();
    let ui = Rc::clone(ui);
    dialog.open(
        Some(&ui.window),
        None::<&gtk::gio::Cancellable>,
        move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    inspect_image(&ui, &path);
                }
            }
        },
    );
}

fn inspect_image(ui: &Ui, path: &Path) {
    let path_text = path.display().to_string();
    match rpc(ui, "images.inspect", json!({ "path": path_text })) {
        Ok(value) => {
            let display = value["display_name"].as_str().unwrap_or("Unknown image");
            let coverage = value["coverage"].as_str().unwrap_or("No coverage metadata");
            let warnings: Vec<&str> = value["warnings"]
                .as_array()
                .map(|items| items.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            ui.image_label.set_label(&format!("{display}\n{coverage}"));
            ui.image_warning.set_label(&warnings.join("  "));
            ui.image_warning.set_visible(!warnings.is_empty());
            *ui.image_path.borrow_mut() = Some(path_text);
            set_status(ui, "Image validated.");
        }
        Err(err) => {
            set_status(ui, &format!("Image check failed: {err}"));
            toast(ui, "Image check failed.");
        }
    }
}

fn choose_data_dialog(ui: &Rc<Ui>) {
    let dialog = gtk::FileDialog::builder()
        .title("Choose a TIS2000 cosids folder")
        .build();
    let ui = Rc::clone(ui);
    dialog.select_folder(
        Some(&ui.window),
        None::<&gtk::gio::Cancellable>,
        move |result| {
            if let Ok(folder) = result {
                if let Some(path) = folder.path() {
                    let text = path.display().to_string();
                    ui.data_label.set_label(&text);
                    *ui.data_dir.borrow_mut() = Some(text);
                    set_status(&ui, "Data folder selected.");
                }
            }
        },
    );
}

fn probe_data(ui: &Ui) {
    let Some(dir) = ui.data_dir.borrow().clone() else {
        toast(ui, "Choose a TIS2000 data folder first.");
        return;
    };
    let tables = rpc(ui, "tis.tables", json!({ "cosids": dir }));
    let probes = rpc(ui, "tis.probe", json!({ "cosids": dir }));
    match (tables, probes) {
        (Ok(tables), Ok(probes)) => {
            let table_count = tables["count"].as_u64().unwrap_or(0);
            let database_count = probes["databases"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter(|item| item["missing"].as_bool() != Some(true))
                        .count()
                })
                .unwrap_or(0);
            ui.data_label.set_label(&format!(
                "{table_count} Access tables; {database_count} SuperBase databases"
            ));
            set_status(ui, "Data probes completed.");
        }
        (Err(err), _) => {
            ui.data_label
                .set_label(&format!("Access probe failed: {err}"));
            set_status(ui, &format!("Data probe failed: {err}"));
        }
        (_, Err(err)) => {
            ui.data_label
                .set_label(&format!("SuperBase probe failed: {err}"));
            set_status(ui, &format!("Data probe failed: {err}"));
        }
    }
}

fn write_card(ui: &Ui) {
    let image = ui.image_path.borrow().clone();
    let port = ui.ports.borrow().first().cloned();
    match (image, port) {
        (Some(image), Some(port)) => {
            match rpc(ui, "card.write", json!({ "port": port, "image": image })) {
                Ok(_) => toast(ui, "Write started (unexpected)."),
                Err(err) => {
                    set_status(ui, &format!("Card write unavailable: {err}"));
                    toast(ui, "Native card writer is not implemented yet.");
                }
            }
        }
        _ => toast(ui, "Choose an image and a serial port first."),
    }
}
