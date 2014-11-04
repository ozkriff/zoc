#![feature(macro_rules)]
#![feature(phase)]
#![feature(globs)]

#![allow(improper_ctypes)]

// #[phase(plugin, link)] extern crate android_glue;

// #[phase(plugin)] extern crate gl_generator;

extern crate libc;
extern crate time;
extern crate native;

// =============================================================================

use libc::{c_void, int32_t};

// To avoid warning: private type in exported type signature
pub use app::{AndroidApp, NativeActivity};
pub use input::Event;
pub use native_window::NativeWindow;
pub use sensor::Looper;

#[link(name = "log")]
extern {
    pub fn __android_log_write(
        prio: libc::c_int,
        tag: *const libc::c_char,
        text: *const libc::c_char,
    ) -> libc::c_int;
}

struct ToLogWriter;

impl Writer for ToLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::IoResult<()> {
        buf.with_c_str(|message| {
            b"RustAndroidGlueStdouterr".with_c_str(|tag| {
                unsafe {
                    __android_log_write(3, tag, message);
                };
            });
        });
        Ok(())
    }
}

pub fn write_log(message: &str) {
    message.with_c_str(|message| {
        b"RustAndroidGlue".with_c_str(|tag| {
            unsafe { __android_log_write(3, tag, message) };
        });
    });
}

mod app {
    use libc::{c_char, c_int, c_void, int32_t, size_t};

    use input;
    use jni;
    use native_window;
    use sensor;

    /// Opaque structure representing callbacks from the framework makes into a native application.
    struct NativeActivityCallbacks;

    /**
     * This structure defines the native side of an android.app.NativeActivity.    It is created by
     * the framework, and handed to the application's native code as it is being launched.
     */
    pub struct NativeActivity {
        /**
         * Pointer to the callback function table of the native application.    You can set the functions
         * here to your own callbacks.    The callbacks pointer itself here should not be changed; it is
         * allocated and managed for you by the framework.
         */
        #[allow(dead_code)]
        callbacks: *const NativeActivityCallbacks,

        /// The global handle on the process's Java VM.
        pub vm: *const jni::JavaVm,

        /**
         * JNI context for the main thread of the app.    Note that this field can ONLY be used from
         * the main thread of the process; that is, the thread that calls into
         * the NativeActivityCallbacks.
         */
        #[allow(dead_code)]
        env: *const jni::JniEnv,

        /// The NativeActivity object handle.
        #[allow(dead_code)]
        activity: *const jni::Jobject,

        /// Path to this application's internal data directory.
        #[allow(dead_code)]
        pub internal_data_path: *const c_char,

        /// Path to this application's external (removable/mountable) data directory.
        #[allow(dead_code)]
        pub external_data_path: *const c_char,

        /// The platform's SDK version code.
        #[allow(dead_code)]
        pub sdk_version: int32_t,

        /**
         * This is the native instance of the application.    It is not used by the framework, but can be
         * set by the application to its own instance state.
         */
        #[allow(dead_code)]
        instance: *mut c_void,

        /**
         * Available starting with Honeycomb: path to the directory containing the application's OBB files
         * (if any).    If the app doesn't have any OBB files, this directory may not exist.
         */
        #[allow(dead_code)]
        pub obb_path: *const c_char,
    }

    /// Opaque structure representing Android configuration.
    struct Configuration;

    struct Rect {
        #[allow(dead_code)]
        left: i32,
        #[allow(dead_code)]
        top: i32,
        #[allow(dead_code)]
        right: i32,
        #[allow(dead_code)]
        bottom: i32,
    }

    // This is the interface for the standard glue code of a threaded application.    In this model, the
    // application's code is running in its own thread separate from the main thread of the process.
    // It is not required that this thread be associated with the Java VM, although it will need to be
    // in order to make JNI calls to any Java objects.    Compatible with C.
    pub struct AndroidApp {
        // The application can place a pointer to its own state object here if it likes.
        pub user_data: *const c_void,
        // Fill this in with the function to process main app commands (APP_CMD_*)
        pub on_app_cmd: *const c_void,
        // Fill this in with the function to process input events.    At this point the event has already
        // been pre-dispatched, and it will be finished upon return.    Return 1 if you have handled
        // the event, 0 for any default dispatching.
        pub on_input_event: *const c_void,
        // The NativeActivity object instance that this app is running in.
        pub activity: *const NativeActivity,
        // The current configuration the app is running in.
        #[allow(dead_code)]
        config: *const Configuration,
        // This is the last instance's saved state, as provided at creation time.    It is NULL if there
        // was no state.    You can use this as you need; the memory will remain around until you call
        // android_app_exec_cmd() for APP_CMD_RESUME, at which point it will be freed and savedState
        // set to NULL.    These variables should only be changed when processing a APP_CMD_SAVE_STATE,
        // at which point they will be initialized to NULL and you can malloc your state and place
        // the information here.    In that case the memory will be freed for you later.
        pub saved_state: *mut c_void,
        pub saved_state_size: size_t,
        // The looper associated with the app's thread.
        pub looper: *const sensor::Looper,
        // When non-NULL, this is the input queue from which the app will receive user input events.
        #[allow(dead_code)]
        input_queue: *const input::Queue,
        // When non-NULL, this is the window surface that the app can draw in.
        pub window: *const native_window::NativeWindow,
        // Current content rectangle of the window; this is the area where the window's content should be
        // placed to be seen by the user.
        #[allow(dead_code)]
        content_rect: Rect,
        // Current state of the app's activity.    May be either APP_CMD_START, APP_CMD_RESUME,
        // APP_CMD_PAUSE, or APP_CMD_STOP; see below.
        #[allow(dead_code)]
        activity_state: c_int,
        // This is non-zero when the application's NativeActivity is being destroyed and waiting for
        // the app thread to complete.
        pub destroy_requested: c_int,
        // Plus some private implementation details.
    }

    // Native app glue command enums:
    pub const CMD_INIT_WINDOW: int32_t = 1;
    pub const CMD_TERM_WINDOW: int32_t = 2;
    pub const CMD_GAINED_FOCUS: int32_t = 6;
    pub const CMD_LOST_FOCUS: int32_t = 7;
    pub const CMD_SAVE_STATE: int32_t = 12;

    /**
     * Data associated with an Looper fd that will be returned as the "data" when that source has
     * data ready.
     */
    pub struct AndroidPollSource {
        /// The identifier of this source.    May be LOOPER_ID_MAIN or LOOPER_ID_INPUT.
        #[allow(dead_code)]
        id: int32_t,
        /// The android_app this ident is associated with.
        #[allow(dead_code)]
        app: *const AndroidApp,
        /// Function to call to perform the standard processing of data from this source.
        pub process: extern "C" fn (app: *mut AndroidApp, source: *const AndroidPollSource),
    }
}

mod egl {
    use libc::{c_uint, c_void};
    use std::ptr;
    use std::vec::Vec;

    use native_window;

    // TODO: Figure out how to put macros in a separate module and import when needed.

    /// Logs the error to Android error logging and fails.
    macro_rules! a_fail(
        ($msg: expr) => ({
            log::e($msg);
            panic!();
        });
        ($fmt: expr, $($arg:tt)*) => ({
            log::e_f(format!($fmt, $($arg)*));
            panic!();
        });
    )

    pub type Display = *const c_void;
    pub const NO_DISPLAY: Display = 0 as Display;

    type NativeDisplayType = *const c_void;
    pub const DEFAULT_DISPLAY: NativeDisplayType = 0 as NativeDisplayType;

    pub type Surface = *const c_void;
    pub const NO_SURFACE: Surface = 0 as Surface;
    pub type Context = *const c_void;
    pub const NO_CONTEXT: Context = 0 as Context;

    pub type Config = *const c_void;

    // Config attributes.
    pub const BLUE_SIZE: Int = 0x3022;
    pub const GREEN_SIZE: Int = 0x3023;
    pub const RED_SIZE: Int = 0x3024;
    pub const DEPTH_SIZE: Int = 0x3025;
    pub const NONE: Int =    0x3038;    /* Attrib list terminator */
    pub const RENDERABLE_TYPE: Int = 0x3040;
    pub const OPENGL_ES2_BIT: Int = 0x0004;    /* EGL_RENDERABLE_TYPE mask bits */
    pub const NATIVE_VISUAL_ID: Int = 0x302E;

    // Context attributes.
    pub const CONTEXT_CLIENT_VERSION: Int = 0x3098;

    type NativeWindowType = *const native_window::NativeWindow;

    type Int = i32;

    // Error codes.
    type Boolean = c_uint;
    // const FALSE: Boolean = 0;
    const TRUE: Boolean = 1;

    pub fn get_display(display_id: NativeDisplayType) -> Display {
        unsafe {
            eglGetDisplay(display_id)
        }
    }

    pub fn initialize(display: Display) {
        let res = unsafe {
            eglInitialize(display, ptr::null_mut(), ptr::null_mut())
        };
        assert!(res == TRUE);
    }

    pub fn choose_config(display: Display, attribs: &[Int], configs: &mut Vec<Config>) {
        let mut num_config: Int = 0;
        let res = unsafe {
            eglChooseConfig(display, attribs.as_ptr(), configs.as_mut_ptr(), configs.len() as Int, &mut num_config)
        };
        assert!(res == TRUE);
        configs.truncate(num_config as uint);
    }

    pub fn get_config_attrib(display: Display, config: Config, attribute: Int) -> Int {
        let mut result: Int = 0;
        let res = unsafe {
            eglGetConfigAttrib(display, config, attribute, &mut result)
        };
        assert!(res == TRUE);
        result
    }

    pub fn create_window_surface(
        display: Display, config: Config, window: NativeWindowType
    ) -> Surface {
        let res = unsafe {
            eglCreateWindowSurface(display, config, window, ptr::null())
        };
        assert!(res != NO_SURFACE);
        res
    }

    pub fn create_context_with_attribs(
        display: Display,
        config: Config,
        share_context: Context,
        attribs: &[Int],
    ) -> Context {
        let res = unsafe {
            eglCreateContext(display, config, share_context, attribs.as_ptr())
        };
        assert!(res != ptr::null())
        res
    }

    pub fn make_current(display: Display, draw: Surface, read: Surface, context: Context) {
        let res = unsafe {
            eglMakeCurrent(display, draw, read, context)
        };
        assert!(res == TRUE);
    }

    pub fn swap_buffers(display: Display, surface: Surface) {
        let res = unsafe {
            eglSwapBuffers(display, surface)
        };
        assert!(res == TRUE);
    }

    pub fn destroy_context(display: Display, context: Context) {
        let res = unsafe {
            eglDestroyContext(display, context)
        };
        assert!(res == TRUE);
    }

    pub fn destroy_surface(display: Display, surface: Surface) {
        let res = unsafe {
            eglDestroySurface(display, surface)
        };
        assert!(res == TRUE);
    }

    pub fn terminate(display: Display) {
        let res = unsafe {
            eglTerminate(display)
        };
        assert!(res == TRUE);
    }

    extern {
        fn eglGetDisplay(display_id: NativeDisplayType) -> Display;
        fn eglInitialize(display: Display, major: *mut Int, minor: *mut Int) -> Boolean;
        fn eglChooseConfig(display: Display, attrib_list: *const Int, configs: *mut Config,
            config_size: Int, num_config: *mut Int) -> Boolean;
        fn eglGetConfigAttrib(display: Display, config: Config, attribute: Int, value: *mut Int) -> Boolean;
        fn eglCreateWindowSurface(display: Display, config: Config, window: NativeWindowType, attrib_list: *const Int) -> Surface;
        fn eglCreateContext(display: Display, config: Config, share_context: Context, attrib_list: *const Int) -> Context;
        fn eglMakeCurrent(display: Display, draw: Surface, read: Surface, context: Context) -> Boolean;
        fn eglSwapBuffers(display: Display, surface: Surface) -> Boolean;
        fn eglDestroyContext(display: Display, context: Context) -> Boolean;
        fn eglDestroySurface(display: Display, surface: Surface) -> Boolean;
        fn eglTerminate(display: Display) -> Boolean;
    }
}

mod gl {
    use libc::{c_float, c_int, c_uchar, c_uint};

    // TODO: Figure out how to put macros in a separate module and import when needed.

    /// Logs the error to Android error logging and fails.
    macro_rules! a_fail(
        ($msg: expr) => ({
            log::e($msg);
            panic!();
        });
        ($fmt: expr, $($arg:tt)*) => ({
            log::e_f(format!($fmt, $($arg)*));
            panic!();
        });
    )

    pub type Enum = c_uint;

    // Error codes.
    const NO_ERROR: Enum = 0;

    type Clampf = c_float;
    type Bitfield = c_uint;
    type Int = c_int;
    type Boolean = c_uchar;

    // glClear mask bits:
    pub const COLOR_BUFFER_BIT: Enum = 0x00004000;

    pub fn clear_color(red: Clampf, green: Clampf, blue: Clampf, alpha: Clampf) {
        unsafe {
            glClearColor(red, green, blue, alpha);
        }
    }

    pub fn clear(mask: Bitfield) {
        unsafe {
            glClear(mask);
        }
        let err = unsafe { glGetError() };
        assert!(err == NO_ERROR);
    }

    extern {
        fn glGetError() -> Enum;
        fn glClearColor(red: Clampf, green: Clampf, blue: Clampf, alpha: Clampf);
        fn glClear(mask: Bitfield);
    }
}

mod input {
    use libc::{c_float, int32_t, size_t};

    use log;

    // TODO: Figure out how to put macros in a separate module and import when needed.

    /// Logs the error to Android error logging and fails.
    macro_rules! a_fail(
        ($msg: expr) => ({
            log::e($msg);
            panic!();
        });
        ($fmt: expr, $($arg:tt)*) => ({
            log::e_f(format!($fmt, $($arg)*));
            panic!();
        });
    )

    /// Input event is an opaque structure.
    pub struct Event;
    /// Input queue is for retrieving input events.
    pub struct Queue;

    // Input event types:
    const EVENT_TYPE_KEY: int32_t = 1;
    const EVENT_TYPE_MOTION: int32_t = 2;
    pub enum EventType {
        Key,
        Motion,
    }

    /// Get the input event type.
    pub fn get_event_type(event: *const Event) -> EventType {
        let res = unsafe {
            AInputEvent_getType(event)
        };
        match res {
            EVENT_TYPE_KEY => Key,
            EVENT_TYPE_MOTION => Motion,
            _ => a_fail!("Unknown event type: {}", res),
        }
    }

    /** Get the current X coordinate of this event for the given pointer index.
     * Whole numbers are pixels; the value may have a fraction for input devices
     * that are sub-pixel precise. */
    pub fn get_motion_event_x(event: *const Event, pointer_index: u32) -> f32 {
        unsafe {
            AMotionEvent_getX(event, pointer_index)
        }
    }

    /* Get the current Y coordinate of this event for the given pointer index.
     * Whole numbers are pixels; the value may have a fraction for input devices
     * that are sub-pixel precise. */
    pub fn get_motion_event_y(event: *const Event, pointer_index: u32) -> f32 {
        unsafe {
            AMotionEvent_getY(event, pointer_index)
        }
    }

    extern {
     fn AInputEvent_getType(event: *const Event) -> int32_t;
     fn AMotionEvent_getX(event: *const Event, pointer_index: size_t) -> c_float;
     fn AMotionEvent_getY(event: *const Event, pointer_index: size_t) -> c_float;
    }
}

mod native_window {
    // Opaque struct for Android native window.
    pub struct NativeWindow;

    pub fn set_buffers_geometry(window: *const NativeWindow, width: i32, height: i32, format: i32) -> i32 {
        unsafe {
            ANativeWindow_setBuffersGeometry(window, width, height, format)
        }
    }

    extern {
        fn ANativeWindow_setBuffersGeometry(window: *const NativeWindow, width: i32, height: i32, format: i32) -> i32;
    }
}

mod log {
    #![macro_escape]

    use libc::{c_char, c_int};

    /// Logs the error to Android error logging and fails.
    macro_rules! a_fail(
        ($msg: expr) => ({
            log::e($msg);
            panic!();
        });
        ($fmt: expr, $($arg:tt)*) => ({
            log::e_f(format!($fmt, $($arg)*));
            panic!();
        });
    )

    /// Logs to Android info logging.
    macro_rules! a_info(
        ($msg: expr) => ( log::i($msg); );
        ($fmt: expr, $($arg:tt)*) => (
            log::i_f(format!($fmt, $($arg)*));
        );
    )

    // Logging priorities:
    const INFO: c_int = 4;
    const ERROR: c_int = 6;

    // Bridges to Android logging at various priorities.
    pub fn i(msg: &str) {
        let c_string = msg.to_c_str();
        unsafe {
            c_log_string(INFO, c_string.as_ptr());
        }
    }

    pub fn i_f(msg: String) {
        let c_string = msg.to_c_str();
        unsafe {
            c_log_string(INFO, c_string.as_ptr());
        }
    }

    pub fn e(msg: &str) {
        let c_string = msg.to_c_str();
        unsafe {
            c_log_string(ERROR, c_string.as_ptr());
        }
    }

    pub fn e_f(msg: String) {
        let c_string = msg.to_c_str();
        unsafe {
            c_log_string(ERROR, c_string.as_ptr());
        }
    }

    extern {
        fn c_log_string(priority: c_int, message: *const c_char);
    }
}

mod sensor {
    use libc::{c_float, c_int, c_void, int8_t, int32_t, int64_t, uint8_t};
    use std::default::Default;
    use std::mem;
    use std::ptr;

    use log;

    // C structure contains unions not representable in Rust, so this is just the
    // version as it applies to accelerometer.
    struct Vector {
        #[allow(dead_code)]
        x: c_float,
        #[allow(dead_code)]
        y: c_float,
        #[allow(dead_code)]
        z: c_float,
        #[allow(dead_code)]
        status: int8_t,
        #[allow(dead_code)]
        reserved: [uint8_t, ..3]
    }

    impl Default for Vector {
        fn default() -> Vector {
            Vector { x: 0.0, y: 0.0, z: 0.0, status: 0, reserved: [0, 0, 0] }
        }
    }

    // C structure contains unions not representable in Rust, so this is just the
    // version as it applies to accelerometer.
    pub struct Event {
        #[allow(dead_code)]
        version: int32_t,    /* size_of(Event) */
        #[allow(dead_code)]
        sensor: int32_t,
        #[allow(dead_code)]
        event_type: int32_t,
        #[allow(dead_code)]
        reserved0: int32_t,
        #[allow(dead_code)]
        timestamp: int64_t,
        #[allow(dead_code)]
        acceleration: Vector,
        #[allow(dead_code)]
        reserved1: [int32_t, ..4]
    }

    impl Default for Event {
        fn default() -> Event {
            Event {
                version: mem::size_of::<Event>() as int32_t,
                sensor: 0,
                event_type: 0,
                reserved0: 0,
                timestamp: 0,
                acceleration: Default::default(),
                reserved1: [0, 0, 0, 0],
            }
        }
    }

    // Looper id enums:
    #[allow(dead_code)]
    pub const LOOPER_ID_MAIN: c_int = 1;
    #[allow(dead_code)]
    pub const LOOPER_ID_INPUT: c_int = 2;

    /**
     * A looper is the state tracking an event loop for a thread.    Loopers do not define event
     * structures or other such things; rather they are a lower-level facility to attach one or more
     * discrete objects listening for an event.    An "event" here is simply data available on a file
     * descriptor: each attached object has an associated file descriptor, and waiting for "events"
     * means (internally) polling on all of these file descriptors until one or more of them have data
     * available.
     *
     * A thread can have only one Looper associated with it.
    */
    pub struct Looper;

    /**
     * For callback-based event loops, this is the prototype of the function that is called when a file
     * descriptor event occurs.    It is given the file descriptor it is associated with, a bitmask
     * of the poll events that were triggered (typically ALOOPER_EVENT_INPUT), and the data pointer
     * that was originally supplied.
     *
     * Implementations should return 1 to continue receiving callbacks, or 0 to have this file
     * descriptor and callback unregistered from the looper.
     */
    // This is the right way but could not make passing null pointers work, neither with 0 as ...,
    // nor with None::<..>.
    // type LooperCallback = extern "C" fn (fd: c_int, events: c_int, data: *const c_void) -> c_int;
    #[allow(dead_code)]
    type LooperCallback = *const c_void;

    // Lopper poll result enums:
    /**
     * The poll was awoken using wake() before the timeout expired and no callbacks were executed and
     * no other file descriptors were ready.
     */
    const ALOOPER_POLL_WAKE: c_int = -1;
    /// One or more callbacks were executed.
    #[allow(dead_code)]
    const ALOOPER_POLL_CALLBACK: c_int = -2;
    /// The timeout expired.
    const ALOOPER_POLL_TIMEOUT: c_int = -3;
    /// An error occurred.
    const ALOOPER_POLL_ERROR: c_int = -4;

    struct PollResult {
        pub id: c_int,
        pub fd: c_int,
        pub events: c_int,
        pub data: *const c_void,
    }

    enum PollErrorEnum {
        #[allow(dead_code)]
        PollWake,
        #[allow(dead_code)]
        PollCallback,
        #[allow(dead_code)]
        PollTimeout,
        #[allow(dead_code)]
        PollError,
    }

    /**
     * Waits for events to be available, with optional timeout in milliseconds.    Invokes callbacks for
     * all file descriptors on which an event occurred.    Performs all pending callbacks until all
     * data has been consumed or a file descriptor is available with no callback.
     *
     * If the timeout is zero, returns immediately without blocking.    If the timeout is negative, waits
     * indefinitely until an event appears.
     *
     * Returns ALOOPER_POLL_WAKE if the poll was awoken using wake() before the timeout expired and
     * no callbacks were invoked and no other file descriptors were ready.
     *
     * Never returns ALOOPER_POLL_CALLBACK.
     *
     * Returns ALOOPER_POLL_TIMEOUT if there was no data before the given timeout expired.
     *
     * Returns ALOOPER_POLL_ERROR if an error occurred.
     *
     * Returns a value >= 0 containing an identifier if its file descriptor has data and it has
     * no callback function (requiring the caller here to handle it).    In this (and only this) case
     * out_fd, out_events and out_data will contain the poll events and data associated with the fd,
     * otherwise they will be set to NULL.
     *
     * This method does not return until it has finished invoking the appropriate callbacks for all
     * file descriptors that were signalled.
     */
    pub fn poll_all(timeout_millis: c_int) -> Result<PollResult, PollErrorEnum> {
        let mut fd: c_int = 0;
        let mut events: c_int = 0;
        let mut data: *const c_void = ptr::null();
        let res = unsafe {
            ALooper_pollAll(timeout_millis, &mut fd as *mut c_int, &mut events as *mut c_int,
                &mut data as *mut *const c_void)
        };
        match res {
            ALOOPER_POLL_WAKE => Err(PollWake),
            ALOOPER_POLL_TIMEOUT => Err(PollTimeout),
            ALOOPER_POLL_ERROR => Err(PollError),
            id if id >= 0 => Ok(PollResult { id: id, fd: fd, events: events, data: data }),
            err => a_fail!("Unknown error from ALooper_pollAll(): {}", err),
        }
    }

    extern {
        fn ALooper_pollAll(timeout_millis: c_int, out_fd: *mut c_int, out_events: *mut c_int, out_data: *mut *const c_void) -> c_int;
    }
}

mod jni {
    use libc::{c_void, int32_t};

    /// JNI invocation interface.
    pub struct JavaVm {
        #[allow(dead_code)]
        functions: *const JniInvokeInterface,
    }

    pub struct JniInvokeInterface {
        #[allow(dead_code)]
        reserved0: *const c_void,
        #[allow(dead_code)]
        reserved1: *const c_void,
        #[allow(dead_code)]
        reserved2: *const c_void,
        #[allow(dead_code)]
        destroy_java_vm: extern fn(*const JavaVm) -> int32_t,
        #[allow(dead_code)]
        attach_current_thread: extern fn(*const JavaVm, *mut *const JniEnv, *const c_void) -> int32_t,
        #[allow(dead_code)]
        detach_current_thread: extern fn(*const JavaVm) -> int32_t,
        #[allow(dead_code)]
        get_env: extern fn(*const JavaVm, *mut *const c_void, int32_t) -> int32_t,
        #[allow(dead_code)]
        attach_current_thread_as_daemon: extern fn(*const JavaVm, *mut *const JniEnv, *const c_void) -> int32_t,
    }

    /// Opaque structure for the JNI context.
    pub struct JniEnv;

    /// Opaque Java object handle.
    pub struct Jobject;
}

mod engine {
    use std::default::Default;
    use std::ptr;

    use egl;
    use gl;
    use input;
    use jni;
    use log;
    use native_window;

    static mut COLOR_COUNTER: i32 = 0;

    // RAII managed EGL pointers.    Cleaned up automatically via Drop.
    struct EglContext {
        display: egl::Display,
        surface: egl::Surface,
        context: egl::Context,
    }

    impl Default for EglContext {
        fn default() -> EglContext {
            EglContext {
                display: egl::NO_DISPLAY,
                surface: egl::NO_SURFACE,
                context: egl::NO_CONTEXT,
            }
        }
    }

    impl EglContext {
        fn swap_buffers(&self) {
            egl::swap_buffers(self.display, self.surface);
        }
    }

    impl Drop for EglContext {
        fn drop(&mut self) {
            if self.display != egl::NO_DISPLAY {
                egl::make_current(
                    self.display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);
                if self.context != egl::NO_CONTEXT {
                    egl::destroy_context(self.display, self.context);
                    self.context = egl::NO_CONTEXT;
                }
                if self.surface != egl::NO_SURFACE {
                    egl::destroy_surface(self.display, self.surface);
                    self.surface = egl::NO_SURFACE;
                }
                egl::terminate(self.display);
                self.display = egl::NO_DISPLAY;
            }
        }
    }

    // Shared state for our app.
    // TODO: Find a way not to declare all fields public.
    pub struct Engine {
        pub jvm: &'static jni::JavaVm,
        pub animating: bool,
        pub egl_context: Option<Box<EglContext>>,
    }

    impl Engine {
        pub fn init(&mut self, egl_context: Box<EglContext>) {
            self.egl_context = Some(egl_context);
            gl::clear_color(0.0, 0.0, 0.0, 1.0);
        }

        pub fn draw(&mut self) {
            match self.egl_context {
                None => {},    // No display.
                Some(ref egl_context) => {
                    unsafe {
                        match COLOR_COUNTER {
                            0 => gl::clear_color(0.3, 0.0, 0.0, 1.0),
                            30 => gl::clear_color(0.0, 0.3, 0.0, 1.0),
                            60 => gl::clear_color(0.0, 0.0, 0.3, 1.0),
                            _ => if COLOR_COUNTER > 90 { COLOR_COUNTER = -1; }
                        }
                        COLOR_COUNTER += 1;
                        gl::clear(gl::COLOR_BUFFER_BIT);
                    }

                    egl_context.swap_buffers();
                }
            }
        }

        /// Update for time passed and draw a frame.
        pub fn update_draw(&mut self) {
            if self.animating {
                self.draw();
            }
        }

        /// Terminate the engine.
        pub fn term(&mut self) {
            self.animating = false;
            self.egl_context = None;    // This closes the existing context via Drop.
            a_info!("Renderer terminated");
        }

        /// Handle touch and key input.    Return true if you handled event, false for any default handling.
        pub fn handle_input(&mut self, event: &input::Event) -> bool {
            match input::get_event_type(event) {
                input::Key => {
                    a_info!("key");
                    return true;
                },
                input::Motion => {
                    let x = input::get_motion_event_x(event, 0);
                    let y = input::get_motion_event_y(event, 0);
                    a_info!("Touch at ({}, {})", x, y);
                    return true;
                },
            }
        }

        /// Called when window gains input focus.
        pub fn gained_focus(&mut self) {
            self.animating = true;
        }

        /// Active when initialized and has focus.
        pub fn is_active(&self) -> bool {
            self.animating
        }

        /// Called when window loses input focus.
        pub fn lost_focus(&mut self) {
            // Also stop animating.
            self.animating = false;
            self.draw();
        }
    }

    pub fn create_egl_context(window: *const native_window::NativeWindow) -> EglContext {
        let display = egl::get_display(egl::DEFAULT_DISPLAY);

        egl::initialize(display);

        // Here specify the attributes of the desired configuration.    Below, we select an EGLConfig with
        // at least 8 bits per color component compatible with OpenGL ES 2.0.    A very simplified
        // selection process, where we pick the first EGLConfig that matches our criteria.
        let attribs_config = [
            egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
            egl::BLUE_SIZE, 8,
            egl::GREEN_SIZE, 8,
            egl::RED_SIZE, 8,
            egl::DEPTH_SIZE, 24,
            egl::NONE
        ];
        let mut configs = vec!(ptr::null());
        egl::choose_config(display, attribs_config, &mut configs);
        if configs.len() == 0 {
            a_fail!("choose_config() did not find any configurations");
        }
        let config = configs[0];

        // EGL_NATIVE_VISUAL_ID is an attribute of the EGLConfig that is guaranteed to be accepted by
        // ANativeWindow_setBuffersGeometry().    As soon as we picked a EGLConfig, we can safely
        // reconfigure the NativeWindow buffers to match, using EGL_NATIVE_VISUAL_ID.
        let format = egl::get_config_attrib(display, config, egl::NATIVE_VISUAL_ID);

        native_window::set_buffers_geometry(window, 0, 0, format);

        let surface = egl::create_window_surface(display, config, window);

        let attribs_context = [
            egl::CONTEXT_CLIENT_VERSION, 2,
            egl::NONE
        ];
        let context = egl::create_context_with_attribs(
            display, config, egl::NO_CONTEXT, attribs_context);

        egl::make_current(display, surface, surface, context);

        EglContext {
            display: display,
            surface: surface,
            context: context,
        }
    }
}

/// Initialize EGL context for the current display.
fn init_display(app_ptr: *mut app::AndroidApp, engine: &mut engine::Engine) {
    a_info!("Renderer initializing...");
    let start_ns = time::precise_time_ns();
    let window = unsafe { (*app_ptr).window };
    let egl_context = box engine::create_egl_context(window);
    engine.init(egl_context);
    let elapsed_ms = (time::precise_time_ns() - start_ns) as f32 / 1000000.0;
    a_info!("Renderer initialized, {:.3f}ms", elapsed_ms);
}

/// Process the next input event.
#[no_mangle]
pub extern fn handle_input(app: *mut app::AndroidApp, event_ptr: *const input::Event) -> int32_t {
    let engine_ptr = unsafe { (*app).user_data as *mut engine::Engine };
    if engine_ptr.is_null() {
        a_fail!("Engine pointer is null");
    }
    let engine: &mut engine::Engine = unsafe { &mut *engine_ptr };
    let event: &input::Event = unsafe { &*event_ptr };
    match engine.handle_input(event) {
        true => 1,
        false => 0,
    }
}

/// Process the next main command.
// Application lifecycle: APP_CMD_START, APP_CMD_RESUME, APP_CMD_INPUT_CHANGED,
// APP_CMD_INIT_WINDOW, APP_CMD_GAINED_FOCUS, ...,
// APP_CMD_SAVE_STATE, APP_CMD_PAUSE, APP_CMD_LOST_FOCUS, APP_CMD_TERM_WINDOW,
// APP_CMD_STOP.
#[no_mangle]
pub extern fn handle_cmd(app_ptr: *mut app::AndroidApp, command: int32_t) {
    let engine_ptr = unsafe { (*app_ptr).user_data as *mut engine::Engine };
    if engine_ptr.is_null() {
        a_fail!("Engine pointer is null");
    }
    let engine: &mut engine::Engine = unsafe { &mut *engine_ptr };

    match command {
        app::CMD_INIT_WINDOW => {
            // The window is being shown, get it ready.
            if unsafe { !(*app_ptr).window.is_null() } {
                init_display(app_ptr, engine);
                engine.draw();
            }
        },
        app::CMD_TERM_WINDOW => {
            // The window is being hidden or closed, clean it up.
            engine.term();
        },
        app::CMD_GAINED_FOCUS => {
            engine.gained_focus();
        },
        app::CMD_LOST_FOCUS => {
            engine.lost_focus();
        },
        app::CMD_SAVE_STATE => {},
        _ => (),
    }
}

fn rust_event_loop(app_ptr: *mut app::AndroidApp, engine_ptr: *mut engine::Engine) {
    let app: &mut app::AndroidApp = unsafe { &mut *app_ptr };
    let engine: &mut engine::Engine = unsafe { &mut *engine_ptr };

    // Loop waiting for stuff to do.
    loop {
        'inner: loop {
            // Block polling when not animating.
            let poll_timeout = if engine.is_active() { 0 } else { -1 };
            match sensor::poll_all(poll_timeout) {
                Err(_) => break 'inner,
                Ok(poll_result) => {
                    // Process this event.
                    if !poll_result.data.is_null() {
                        let source: &app::AndroidPollSource = unsafe {
                            &*(poll_result.data as *const app::AndroidPollSource)
                        };
                        let process = source.process;
                        process(app_ptr, source as *const app::AndroidPollSource);
                    }

                    /*
                    // If the sensor has data, process it now.
                    if poll_result.id == sensor::LOOPER_ID_USER {
                        engine.handle_sensor_events();
                    }
                    */

                    // Check if should exit.
                    if app.destroy_requested != 0 {
                        engine.term();
                        return;
                    }
                }
            }
        }
        engine.update_draw();
    }
}

/**
 * This is the main entry point of a native application that is using android_native_app_glue.
 * It runs in its own thread, with its own event loop for receiving input events and doing other
 * things.
 */
#[no_mangle]
pub extern fn glue_main(app_ptr: *mut app::AndroidApp) {
    a_info!("-------------------------------------------------------------------");

    let app: &mut app::AndroidApp = unsafe { &mut *app_ptr };
    let activity: &app::NativeActivity = unsafe { &*app.activity };
    let jvm: &jni::JavaVm = unsafe { &*activity.vm };

    let mut engine = engine::Engine {
        jvm: jvm,
        animating: false,
        egl_context: None,
    };

    // Notify the system about our custom data and callbacks.
    app.user_data = &engine as *const engine::Engine as *const c_void;
    app.on_app_cmd = handle_cmd as *const c_void;
    app.on_input_event = handle_input as *const c_void;

    rust_event_loop(app_ptr, &mut engine as *mut engine::Engine);
}


// =============================================================================

/*
fn main(app: *mut()) {
    println!("start");
    glue_main(app as *mut app::AndroidApp);
}
*/

#[no_mangle]
pub fn rust_android_main(app: *mut()) {
    native::start(1, &b"".as_ptr(), proc() {
        // android_glue::android_main2(app, proc() main(app));
        std::io::stdio::set_stdout(box std::io::LineBufferedWriter::new(ToLogWriter));
        std::io::stdio::set_stderr(box std::io::LineBufferedWriter::new(ToLogWriter));
        // panic!("FUCK");
        println!("println: test");
        write_log("write_log: test");
        glue_main(app as *mut app::AndroidApp);
    });
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
