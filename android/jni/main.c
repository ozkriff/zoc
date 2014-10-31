#include <android_native_app_glue.h>

/*void c_log_string(int priority, const char *message) {
  __android_log_write(priority, "native-activity", message);
}*/

/* Function implemented in Rust. */
extern void rust_android_main(struct android_app* app);

void android_main(struct android_app* app) {
  app_dummy(); /* Make sure glue isn't stripped. */
  rust_android_main(app);
}

/* TODO: link to libportable */
void _Unwind_GetIP() {};
void _Unwind_SetIP() {};
void _Unwind_SetGR() {};
void _Unwind_GetGR() {};
