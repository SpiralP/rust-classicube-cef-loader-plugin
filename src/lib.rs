mod async_manager;
mod cef_binary_updater;
mod error;
mod github_release_checker;
mod loader;
mod logger;
mod plugin_updater;

use crate::{async_manager::AsyncManager, plugin_updater::update_plugins};
use classicube_sys::{
    Chat_Add, Chat_AddOf, IGameComponent, MsgType_MSG_TYPE_CLIENTSTATUS_2, OwnedString,
};
use log::debug;
use std::{cell::RefCell, os::raw::c_int, ptr};

thread_local!(
    static ASYNC_MANAGER: RefCell<Option<AsyncManager>> = RefCell::new(None);
);

extern "C" fn init() {
    logger::initialize(true, false);

    debug!("Init");

    ASYNC_MANAGER.with(|cell| {
        let option = &mut *cell.borrow_mut();
        let mut async_manager = AsyncManager::new();
        async_manager.initialize();
        *option = Some(async_manager);
    });

    // never update if debug build
    #[cfg(not(debug_assertions))]
    update_plugins();

    #[cfg(debug_assertions)]
    if false {
        update_plugins();
    }

    loader::init();
}

extern "C" fn free() {
    debug!("Free");

    loader::free();

    ASYNC_MANAGER.with(|cell| {
        if let Some(mut async_manager) = cell.borrow_mut().take() {
            async_manager.shutdown();
        }
    });
}

extern "C" fn reset() {
    debug!("Reset");

    loader::reset();
}

extern "C" fn on_new_map() {
    debug!("OnNewMap");

    loader::on_new_map();
}

extern "C" fn on_new_map_loaded() {
    debug!("OnNewMapLoaded");

    loader::on_new_map_loaded();

    // TODO fix this!!
    // I want to unload this plugin's async runtimes once we're done with updating
    // if SHOULD_SHUTDOWN.with(|cell| cell.get()) {
    //     SHOULD_SHUTDOWN.with(|cell| cell.set(false));
    //     ASYNC_MANAGER.with(|cell| {
    //         if let Some(mut async_manager) = cell.borrow_mut().take() {
    //             async_manager.shutdown();
    //         }
    //     });
    // }
}

#[no_mangle]
pub static Plugin_ApiVersion: c_int = 1;

#[no_mangle]
pub static mut Plugin_Component: IGameComponent = IGameComponent {
    // Called when the game is being loaded.
    Init: Some(init),
    // Called when the component is being freed. (e.g. due to game being closed)
    Free: Some(free),
    // Called to reset the component's state. (e.g. reconnecting to server)
    Reset: Some(reset),
    // Called to update the component's state when the user begins loading a new map.
    OnNewMap: Some(on_new_map),
    // Called to update the component's state when the user has finished loading a new map.
    OnNewMapLoaded: Some(on_new_map_loaded),
    // Next component in linked list of components.
    next: ptr::null_mut(),
};

pub fn print<S: Into<String>>(s: S) {
    let s = s.into();
    debug!("{}", s);

    let owned_string = OwnedString::new(s);

    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub fn status<S: Into<String>>(s: S) {
    let s = s.into();
    debug!("{}", s);

    let owned_string = OwnedString::new(s);

    unsafe {
        Chat_AddOf(
            owned_string.as_cc_string(),
            MsgType_MSG_TYPE_CLIENTSTATUS_2 as _,
        );
    }
}

pub async fn print_async<S: Into<String> + Send + 'static>(s: S) {
    AsyncManager::run_on_main_thread(async move {
        print(s);
    })
    .await;
}
