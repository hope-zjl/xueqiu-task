#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use notify_rust::Notification;
use regex::Regex;
use rss::Channel;
// 导入正确的类型
use slint::{ComponentHandle, Timer, ToSharedString, Weak};

slint::include_modules!();

// 拖动状态：调整类型为i32（物理坐标是整数）
// (初始窗口物理x/y, 按下时鼠标逻辑x/y, 最后更新时间)
type DragState = Arc<Mutex<Option<(i32, i32, f32, f32, Instant)>>>;
const DEAD_ZONE: f32 = 0.1;

fn main() -> Result<(), slint::PlatformError> {
    setup_and_run_app()
}

fn setup_and_run_app() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let weak_window = main_window.as_weak();
    setup_callbacks(&main_window, &weak_window);
    windows_conse(&main_window);

    let drag_state: DragState = Arc::new(Mutex::new(None));

    // === start-drag 回调（修正类型：物理坐标是i32）===
    let state_clone = drag_state.clone();
    let weak_for_start = weak_window.clone();
    main_window.on_start_drag(move |mouse_down_x, mouse_down_y| {
        if let Some(window) = weak_for_start.upgrade() {
            // 物理位置本身就是i32，直接获取
            let physical_pos = window.window().position();
            *state_clone.lock().unwrap() = Some((
                physical_pos.x, // i32
                physical_pos.y, // i32
                mouse_down_x,   // f32（逻辑坐标）
                mouse_down_y,   // f32（逻辑坐标）
                Instant::now(),
            ));
        }
    });

    let weak_for_move = weak_window.clone();
    main_window.on_move_window(move |dx, dy| {
        if dx.abs() < DEAD_ZONE && dy.abs() < DEAD_ZONE {
            return;
        }

        if let Some(window) = weak_for_move.upgrade() {
            // 获取当前窗口的逻辑位置
            let current_logical = window
                .window()
                .position()
                .to_logical(window.window().scale_factor());

            // 计算新的逻辑位置
            let new_logical_x = current_logical.x + dx;
            let new_logical_y = current_logical.y + dy;

            // 直接设置（Slint 会自动转为物理位置）
            window
                .window()
                .set_position(slint::LogicalPosition::new(new_logical_x, new_logical_y));
        }
    });

    let _timer_time = get_current_date(&main_window);

    let weak_for_weather = weak_window.clone();
    thread::spawn(move || {
        // 这里是普通的同步代码，没有 .await！
        let result = fetch_weather_blocking(); // <-- 新函数

        // 通过 Slint 的 invoke_from_event_loop 回到主线程更新 UI
        slint::invoke_from_event_loop(move || {
            if let Some(window) = weak_for_weather.upgrade() {
                match result {
                    Ok(weather_text) => {
                        // 假设你有一个 weather_info 属性
                        let re = Regex::new(r"溫度:\s*([0-9]+\s*~\s*[0-9]+).*?降雨機率:\s*(\d+%)")
                            .unwrap();
                        if let Some(caps) = re.captures(weather_text.as_str()) {
                            let temperature = caps.get(1).map(|m| m.as_str().to_string()).unwrap();
                            let rain_prob = caps.get(2).map(|m| m.as_str().to_string()).unwrap();
                            let result = format!("温度:{},降水概率:{}",temperature, rain_prob);
                            window.set_weather_info(result.into());
                        } else {
                            window.set_weather_info("正则失败".into());
                        }
                    }
                    Err(e) => {
                        eprintln!("获取天气失败: {}", e);
                        window.set_weather_info("加载失败".into());
                    }
                }
            }
        })
        .unwrap();
    });

    main_window.run()
}

// 以下代码完全不变
fn setup_callbacks(window: &MainWindow, _weak_window: &Weak<MainWindow>) {
    window.on_timer_finished(move || {
        Notification::new()
            .summary("雪球")
            .body("定时任务结束！")
            .icon("thunderbird")
            .appname("thunderbird")
            .sound_name("Alarm")
            .timeout(0)
            .show()
            .unwrap();
    });
}

fn get_current_date(window: &MainWindow) -> Timer {
    let timer = Timer::default();
    let windw_weak = window.as_weak();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(1),
        move || {
            if let Some(ui) = windw_weak.upgrade() {
                let now = chrono::Local::now().format("%H:%M:%S").to_shared_string();
                ui.set_current_time(now);
            }
        },
    );
    timer
}

fn windows_conse(window: &MainWindow) {
    let weak_window = window.as_weak();
    window.on_window_colse(move || {
        if let Some(ui) = weak_window.upgrade() {
            ui.hide().unwrap();
        }
        std::process::exit(0);
    });
}

fn fetch_weather_blocking() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://www.cwa.gov.tw/rss/forecast/36_01.xml";

    // 使用 reqwest 的 blocking 客户端
    let client = reqwest::blocking::Client::new();
    let response = client.get(url).send()?; // 同步调用
    let content = response.bytes()?; // 同步获取字节

    // 解析 RSS
    let channel = Channel::read_from(&content[..])?;

    // 提取信息
    if let Some(item) = channel.items().first() {
        if let Some(title) = item.title() {
            return Ok(title.to_string());
        }
    }

    Ok("暂无数据".to_string())
}
