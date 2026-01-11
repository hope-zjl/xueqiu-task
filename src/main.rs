#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    sync::{Arc, Mutex},
    time::Duration,
    time::Instant,
};

use notify_rust::Notification;
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
