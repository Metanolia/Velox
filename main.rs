use gtk::prelude::*;
use gtk::glib;
use webkit6::prelude::*;
use webkit6::{WebView, NetworkSession};

fn main() {
    gtk::init().expect("Не удалось инициализировать GTK");

    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
    }

    let app = gtk::Application::builder()
    .application_id("com.velox.browser")
    .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &gtk::Application) {
    let session = NetworkSession::new_ephemeral();

    let window = gtk::ApplicationWindow::builder()
    .application(app)
    .title("Velox")
    .default_width(1280)
    .default_height(800)
    .build();

    // ─── Главный контейнер ────────────────────────────────────────────────
    let main_box = gtk::Box::builder()
    .orientation(gtk::Orientation::Vertical)
    .spacing(0)
    .build();

    // ─── Панель навигации ─────────────────────────────────────────────────
    let toolbar = gtk::Box::builder()
    .orientation(gtk::Orientation::Horizontal)
    .spacing(4)
    .margin_top(6)
    .margin_bottom(6)
    .margin_start(6)
    .margin_end(6)
    .build();

    let back_btn    = gtk::Button::from_icon_name("go-previous-symbolic");
    let forward_btn = gtk::Button::from_icon_name("go-next-symbolic");
    let reload_btn  = gtk::Button::from_icon_name("view-refresh-symbolic");
    let home_btn    = gtk::Button::from_icon_name("go-home-symbolic");

    // Отключаем до первой навигации
    back_btn.set_sensitive(false);
    forward_btn.set_sensitive(false);

    let url_entry = gtk::Entry::builder()
    .placeholder_text("Введите адрес или поисковый запрос…")
    .hexpand(true)
    .build();

    toolbar.append(&back_btn);
    toolbar.append(&forward_btn);
    toolbar.append(&reload_btn);
    toolbar.append(&home_btn);
    toolbar.append(&url_entry);

    // ─── Прогресс-бар ─────────────────────────────────────────────────────
    let progress_bar = gtk::ProgressBar::builder()
    .visible(false)
    .build();

    // ─── WebView ───────────────────────────────────────────────────────────
    let webview = WebView::builder()
    .network_session(&session)
    .build();

    webview.set_vexpand(true);
    webview.load_uri("https://example.com");

    // ─── Переход по URL ────────────────────────────────────────────────────
    {
        let wv = webview.clone();
        url_entry.connect_activate(move |entry| {
            let mut url = entry.text().to_string();
            if !url.starts_with("http://") && !url.starts_with("https://") {
                if url.contains('.') && !url.contains(' ') {
                    url = format!("https://{}", url);
                } else {
                    url = format!(
                        "https://www.google.com/search?q={}",
                        url.replace(' ', "+")
                    );
                }
            }
            println!("[Velox] Загрузка: {}", url);
            wv.load_uri(&url);
        });
    }

    // ─── Кнопки назад / вперёд / обновить / домой ────────────────────────────────
    {
        let wv = webview.clone();
        back_btn.connect_clicked(move |_| { wv.go_back(); });
    }
    {
        let wv = webview.clone();
        forward_btn.connect_clicked(move |_| { wv.go_forward(); });
    }
    {
        let wv = webview.clone();
        reload_btn.connect_clicked(move |_| { wv.reload(); });
    }

    {
        let wv = webview.clone();
        home_btn.connect_clicked(move |_| {
            wv.load_uri("https://example.com");
        });
    }

    // ─── Плавный прогресс загрузки ─────────────────────────────────────────
    {
        let pb = progress_bar.clone();
        webview.connect_estimated_load_progress_notify(move |wv| {
            pb.set_fraction(wv.estimated_load_progress());
        });
    }

    // ─── События загрузки ─────────────────────────────────────────────────
    {
        let entry = url_entry.clone();
        let pb    = progress_bar.clone();
        let back  = back_btn.clone();
        let fwd   = forward_btn.clone();

        webview.connect_load_changed(move |wv, event| {
            match event {
                webkit6::LoadEvent::Started => {
                    pb.set_visible(true);
                    pb.set_fraction(0.0);
                    if let Some(uri) = wv.uri() {
                        entry.set_text(uri.as_str());
                    }
                }
                webkit6::LoadEvent::Committed => {
                    if let Some(uri) = wv.uri() {
                        entry.set_text(uri.as_str());
                    }
                }
                webkit6::LoadEvent::Finished => {
                    pb.set_visible(false);
                    if let Some(uri) = wv.uri() {
                        entry.set_text(uri.as_str());
                    }
                    back.set_sensitive(wv.can_go_back());
                    fwd.set_sensitive(wv.can_go_forward());
                }
                _ => {}
            }
        });
    }

    // ─── Ошибка загрузки ──────────────────────────────────────────────────
    {
        let pb = progress_bar.clone();
        webview.connect_load_failed(move |_, _, uri, error| {
            eprintln!("[Velox ERROR] Ошибка загрузки «{}»: {}", uri, error);
            pb.set_visible(false);
            true
        });
    }

    // ─── Заголовок окна ───────────────────────────────────────────────────
    {
        let win = window.clone();
        webview.connect_title_notify(move |wv| {
            let title = wv
            .title()
            .map(|t| format!("{} — Velox", t))
            .unwrap_or_else(|| "Velox".to_string());
            win.set_title(Some(&title));
        });
    }

    // ─── Горячие клавиши ──────────────────────────────────────────────────
    //
    //  Ctrl+L          — фокус на адресную строку (выделить всё)
    //  Ctrl+R / F5     — перезагрузить страницу
    //  Ctrl+Shift+R    — жёсткая перезагрузка (без кэша)
    //  Escape          — остановить загрузку
    //  Alt+←           — назад
    //  Alt+→           — вперёд
    //
    {
        let wv    = webview.clone();
        let entry = url_entry.clone();
        let back  = back_btn.clone();
        let fwd   = forward_btn.clone();

        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, mods| {
            use gtk::gdk::{Key, ModifierType};
            let ctrl  = mods.contains(ModifierType::CONTROL_MASK);
            let shift = mods.contains(ModifierType::SHIFT_MASK);
            let alt   = mods.contains(ModifierType::ALT_MASK);

            // Ctrl+L — фокус на адрес
            if ctrl && (key == Key::l || key == Key::L) {
                entry.grab_focus();
                entry.select_region(0, -1);
                return glib::Propagation::Stop;
            }

            // Ctrl+Shift+R — жёсткая перезагрузка
            if ctrl && shift && (key == Key::r || key == Key::R) {
                wv.reload_bypass_cache();
                return glib::Propagation::Stop;
            }

            // Ctrl+R / F5 — обычная перезагрузка
            if (ctrl && (key == Key::r || key == Key::R)) || key == Key::F5 {
                wv.reload();
                return glib::Propagation::Stop;
            }

            // Escape — остановить
            if key == Key::Escape {
                wv.stop_loading();
                return glib::Propagation::Stop;
            }

            // Alt+← — назад
            if alt && key == Key::Left {
                if back.is_sensitive() { wv.go_back(); }
                return glib::Propagation::Stop;
            }

            // Alt+→ — вперёд
            if alt && key == Key::Right {
                if fwd.is_sensitive() { wv.go_forward(); }
                return glib::Propagation::Stop;
            }

            // Alt+Home или Ctrl+H - домой
            if (alt && key == Key::Home) || (ctrl && (key == Key::h || key == Key::H)) {
                wv.load_uri("https://example.com");
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });

        window.add_controller(controller);
    }

    // ─── Сборка UI ────────────────────────────────────────────────────────
    main_box.append(&toolbar);
    main_box.append(&progress_bar);
    main_box.append(&webview);
    window.set_child(Some(&main_box));
    window.present();
}
