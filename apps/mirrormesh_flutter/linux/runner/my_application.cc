#include "my_application.h"

#include <cstdint>
#include <cstddef>
#include <memory>

#include <flutter_linux/flutter_linux.h>
#include <flutter_linux/fl_method_call.h>
#include <flutter_linux/fl_method_channel.h>
#include <flutter_linux/fl_method_response.h>
#include <flutter_linux/fl_standard_method_codec.h>
#include <flutter_linux/fl_value.h>
#ifdef GDK_WINDOWING_X11
#include <gdk/gdkx.h>
#endif

#include "flutter/generated_plugin_registrant.h"

extern "C" int32_t mirrormesh_linux_prepare_pairing_runtime(
    uint8_t* device_id_out,
    size_t device_id_capacity);

struct _MyApplication {
  GtkApplication parent_instance;
  char** dart_entrypoint_arguments;
  FlMethodChannel* pairing_channel;
};

G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

struct PairingRuntimeReply {
  FlMethodCall* method_call;
  int32_t status;
  uint8_t device_id[16];
};

static gboolean respond_pairing_runtime(gpointer user_data) {
  std::unique_ptr<PairingRuntimeReply> reply(
      static_cast<PairingRuntimeReply*>(user_data));
  g_autoptr(FlMethodResponse) response = nullptr;

  if (reply->status == 0) {
    g_autofree gchar* device_id = g_strdup_printf(
        "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
        reply->device_id[0], reply->device_id[1], reply->device_id[2],
        reply->device_id[3], reply->device_id[4], reply->device_id[5],
        reply->device_id[6], reply->device_id[7], reply->device_id[8],
        reply->device_id[9], reply->device_id[10], reply->device_id[11],
        reply->device_id[12], reply->device_id[13], reply->device_id[14],
        reply->device_id[15]);
    g_autoptr(FlValue) result = fl_value_new_string(device_id);
    response = FL_METHOD_RESPONSE(fl_method_success_response_new(result));
  } else {
    response = FL_METHOD_RESPONSE(fl_method_error_response_new(
        "pairing_runtime_unavailable",
        "Linux could not open local pairing storage.", nullptr));
  }

  g_autoptr(GError) error = nullptr;
  if (!fl_method_call_respond(reply->method_call, response, &error)) {
    g_warning("Failed to return Linux pairing status: %s", error->message);
  }
  g_object_unref(reply->method_call);
  return G_SOURCE_REMOVE;
}

static gpointer initialize_pairing_runtime(gpointer user_data) {
  auto* reply = static_cast<PairingRuntimeReply*>(user_data);
  reply->status = mirrormesh_linux_prepare_pairing_runtime(
      reply->device_id, sizeof(reply->device_id));
  g_main_context_invoke(nullptr, respond_pairing_runtime, reply);
  return nullptr;
}

static void pairing_method_call_cb(FlMethodChannel* channel,
                                   FlMethodCall* method_call,
                                   gpointer user_data) {
  (void)channel;
  (void)user_data;
  if (g_strcmp0(fl_method_call_get_name(method_call),
                "preparePairingRuntime") != 0) {
    g_autoptr(FlMethodResponse) response =
        FL_METHOD_RESPONSE(fl_method_not_implemented_response_new());
    fl_method_call_respond(method_call, response, nullptr);
    return;
  }

  auto* reply = new PairingRuntimeReply{
      FL_METHOD_CALL(g_object_ref(method_call)), 2, {0}};
  GThread* worker = g_thread_new(
      "mirrormesh-pairing-runtime", initialize_pairing_runtime, reply);
  g_thread_unref(worker);
}

// Called when first Flutter frame received.
static void first_frame_cb(MyApplication* self, FlView* view) {
  gtk_widget_show(gtk_widget_get_toplevel(GTK_WIDGET(view)));
}

// Implements GApplication::activate.
static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));

  // Use a header bar when running in GNOME as this is the common style used
  // by applications and is the setup most users will be using (e.g. Ubuntu
  // desktop).
  // If running on X and not using GNOME then just use a traditional title bar
  // in case the window manager does more exotic layout, e.g. tiling.
  // If running on Wayland assume the header bar will work (may need changing
  // if future cases occur).
  gboolean use_header_bar = TRUE;
#ifdef GDK_WINDOWING_X11
  GdkScreen* screen = gtk_window_get_screen(window);
  if (GDK_IS_X11_SCREEN(screen)) {
    const gchar* wm_name = gdk_x11_screen_get_window_manager_name(screen);
    if (g_strcmp0(wm_name, "GNOME Shell") != 0) {
      use_header_bar = FALSE;
    }
  }
#endif
  if (use_header_bar) {
    GtkHeaderBar* header_bar = GTK_HEADER_BAR(gtk_header_bar_new());
    gtk_widget_show(GTK_WIDGET(header_bar));
    gtk_header_bar_set_title(header_bar, "mirrormesh_flutter");
    gtk_header_bar_set_show_close_button(header_bar, TRUE);
    gtk_window_set_titlebar(window, GTK_WIDGET(header_bar));
  } else {
    gtk_window_set_title(window, "mirrormesh_flutter");
  }

  gtk_window_set_default_size(window, 1280, 720);

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(
      project, self->dart_entrypoint_arguments);

  FlView* view = fl_view_new(project);
  GdkRGBA background_color;
  // Background defaults to black, override it here if necessary, e.g. #00000000
  // for transparent.
  gdk_rgba_parse(&background_color, "#000000");
  fl_view_set_background_color(view, &background_color);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  // Show the window when Flutter renders.
  // Requires the view to be realized so we can start rendering.
  g_signal_connect_swapped(view, "first-frame", G_CALLBACK(first_frame_cb),
                           self);
  gtk_widget_realize(GTK_WIDGET(view));

  fl_register_plugins(FL_PLUGIN_REGISTRY(view));

  g_clear_object(&self->pairing_channel);
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  self->pairing_channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)),
      "com.mirrormesh/linux_pairing", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      self->pairing_channel, pairing_method_call_cb, nullptr, nullptr);

  gtk_widget_grab_focus(GTK_WIDGET(view));
}

// Implements GApplication::local_command_line.
static gboolean my_application_local_command_line(GApplication* application,
                                                  gchar*** arguments,
                                                  int* exit_status) {
  MyApplication* self = MY_APPLICATION(application);
  // Strip out the first argument as it is the binary name.
  self->dart_entrypoint_arguments = g_strdupv(*arguments + 1);

  g_autoptr(GError) error = nullptr;
  if (!g_application_register(application, nullptr, &error)) {
    g_warning("Failed to register: %s", error->message);
    *exit_status = 1;
    return TRUE;
  }

  g_application_activate(application);
  *exit_status = 0;

  return TRUE;
}

// Implements GApplication::startup.
static void my_application_startup(GApplication* application) {
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application startup.

  G_APPLICATION_CLASS(my_application_parent_class)->startup(application);
}

// Implements GApplication::shutdown.
static void my_application_shutdown(GApplication* application) {
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application shutdown.

  G_APPLICATION_CLASS(my_application_parent_class)->shutdown(application);
}

// Implements GObject::dispose.
static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  g_clear_object(&self->pairing_channel);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->local_command_line =
      my_application_local_command_line;
  G_APPLICATION_CLASS(klass)->startup = my_application_startup;
  G_APPLICATION_CLASS(klass)->shutdown = my_application_shutdown;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}

static void my_application_init(MyApplication* self) {}

MyApplication* my_application_new() {
  // Set the program name to the application ID, which helps various systems
  // like GTK and desktop environments map this running application to its
  // corresponding .desktop file. This ensures better integration by allowing
  // the application to be recognized beyond its binary name.
  g_set_prgname(APPLICATION_ID);

  return MY_APPLICATION(g_object_new(my_application_get_type(),
                                     "application-id", APPLICATION_ID, "flags",
                                     G_APPLICATION_NON_UNIQUE, nullptr));
}
