#define _GNU_SOURCE
#include "dodge_native.h"
#include "plasma_window_management.h"
#include "org_kde_plasma_virtual_desktop.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>
#include <errno.h>
#include <stdbool.h>
#include <stdatomic.h>
#include <wayland-client.h>

#define MAX_WINDOWS 256
#define MAX_DESKTOPS 32
#define MAX_WINDOW_DESKTOPS 8
#define DESKTOP_ID_LEN 64

struct WinEntry {
    struct org_kde_plasma_window *win;
    char uuid[64];
    char app_id[128];
    char title[128];
    char desktops[MAX_WINDOW_DESKTOPS][DESKTOP_ID_LEN];
    int desktop_count;
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
    uint32_t pid;
    uint32_t state_flags;
    bool mapped;
    bool minimized;
    bool fullscreen;
    bool on_all_desktops;
    bool desktop_overflow;
    bool in_use;
};

struct DesktopEntry {
    struct org_kde_plasma_virtual_desktop *desktop;
    char id[DESKTOP_ID_LEN];
    bool active;
    bool in_use;
};

struct DodgeContext {
    struct wl_display *display;
    struct wl_registry *registry;
    struct org_kde_plasma_window_management *wm;
    struct org_kde_plasma_virtual_desktop_management *vdm;
    struct WinEntry windows[MAX_WINDOWS];
    struct DesktopEntry desktops[MAX_DESKTOPS];
    bool desktop_overflow;

    int32_t notch_x;
    int32_t notch_y;
    uint32_t notch_w;
    uint32_t notch_h;

    DodgeOverlapCallback callback;
    void *user_data;

    bool last_overlap;
    uint32_t my_pid;
    int wake_pipe[2];
    atomic_bool running;
};

static struct WinEntry *find_window(struct DodgeContext *ctx, struct org_kde_plasma_window *w) {
    if (!ctx || !w) return NULL;
    for (int i = 0; i < MAX_WINDOWS; ++i) {
        if (ctx->windows[i].in_use && ctx->windows[i].win == w) {
            return &ctx->windows[i];
        }
    }
    return NULL;
}

static struct DesktopEntry *find_desktop(struct DodgeContext *ctx, struct org_kde_plasma_virtual_desktop *desktop) {
    if (!ctx || !desktop) return NULL;
    for (int i = 0; i < MAX_DESKTOPS; ++i) {
        if (ctx->desktops[i].in_use && ctx->desktops[i].desktop == desktop) {
            return &ctx->desktops[i];
        }
    }
    return NULL;
}

static struct DesktopEntry *find_desktop_by_id(struct DodgeContext *ctx, const char *id) {
    if (!ctx || !id) return NULL;
    for (int i = 0; i < MAX_DESKTOPS; ++i) {
        if (ctx->desktops[i].in_use && strcmp(ctx->desktops[i].id, id) == 0) {
            return &ctx->desktops[i];
        }
    }
    return NULL;
}

static void add_window_desktop(struct WinEntry *w, const char *id) {
    if (!w || !id || id[0] == '\0') return;
    for (int i = 0; i < w->desktop_count; ++i) {
        if (strcmp(w->desktops[i], id) == 0) return;
    }
    if (w->desktop_count >= MAX_WINDOW_DESKTOPS || strlen(id) >= DESKTOP_ID_LEN) {
        w->desktop_overflow = true;
        return;
    }
    snprintf(w->desktops[w->desktop_count], DESKTOP_ID_LEN, "%s", id);
    w->desktop_count++;
}

static void remove_window_desktop(struct WinEntry *w, const char *id) {
    if (!w || !id) return;
    for (int i = 0; i < w->desktop_count; ++i) {
        if (strcmp(w->desktops[i], id) == 0) {
            for (int j = i; j < w->desktop_count - 1; ++j) {
                memcpy(w->desktops[j], w->desktops[j + 1], DESKTOP_ID_LEN);
            }
            w->desktop_count--;
            w->desktops[w->desktop_count][0] = '\0';
            return;
        }
    }
}

static bool window_on_active_desktop(struct DodgeContext *ctx, struct WinEntry *w) {
    if (!ctx || !w) return true;
    if (w->on_all_desktops || w->desktop_overflow || ctx->desktop_overflow) return true;
    if (w->desktop_count == 0) return true;

    bool has_active = false;
    for (int i = 0; i < MAX_DESKTOPS; ++i) {
        if (ctx->desktops[i].in_use && ctx->desktops[i].active) {
            has_active = true;
            break;
        }
    }
    if (!has_active) return true;

    for (int i = 0; i < w->desktop_count; ++i) {
        for (int j = 0; j < MAX_DESKTOPS; ++j) {
            if (ctx->desktops[j].in_use && ctx->desktops[j].active &&
                strcmp(w->desktops[i], ctx->desktops[j].id) == 0) {
                return true;
            }
        }
    }
    return false;
}

static void check_collisions(struct DodgeContext *ctx) {
    if (!ctx || !ctx->callback) return;

    bool overlapping = false;
    for (int i = 0; i < MAX_WINDOWS; ++i) {
        if (!ctx->windows[i].in_use) continue;
        struct WinEntry *w = &ctx->windows[i];
        if (!w->mapped || w->minimized) continue;
        if (w->pid == ctx->my_pid) continue;
        if (strcmp(w->app_id, "island-app") == 0 ||
            strcmp(w->title, "island-app") == 0 ||
            strcmp(w->app_id, "slint-spike") == 0 ||
            strcmp(w->app_id, "pill-panel") == 0) {
            continue;
        }
        if (!window_on_active_desktop(ctx, w)) continue;

        // Exclude Plasma desktop background, panels, docks, and desktop widgets
        if (strstr(w->app_id, "plasmashell") != NULL ||
            strstr(w->app_id, "plasma") != NULL ||
            strcmp(w->app_id, "krunner") == 0 ||
            strstr(w->title, "Desktop — Plasma") != NULL ||
            strcmp(w->title, "Plasma") == 0) {
            continue;
        }

        // Exclude surfaces marked as skiptaskbar or keep_below (desktop wallpaper/docks)
        if ((w->state_flags & ORG_KDE_PLASMA_WINDOW_MANAGEMENT_STATE_SKIPTASKBAR) != 0 ||
            (w->state_flags & ORG_KDE_PLASMA_WINDOW_MANAGEMENT_STATE_KEEP_BELOW) != 0) {
            continue;
        }

        if (w->width == 0 || w->height == 0) continue;

        int32_t win_r = w->x + (int32_t)w->width;
        int32_t win_b = w->y + (int32_t)w->height;
        int32_t notch_r = ctx->notch_x + (int32_t)ctx->notch_w;
        int32_t notch_b = ctx->notch_y + (int32_t)ctx->notch_h;

        if (w->x < notch_r && win_r > ctx->notch_x &&
            w->y < notch_b && win_b > ctx->notch_y) {
            overlapping = true;
            break;
        }
    }

    if (overlapping != ctx->last_overlap) {
        ctx->last_overlap = overlapping;
        ctx->callback(overlapping, ctx->user_data);
    }
}

static void handle_geometry(void *data, struct org_kde_plasma_window *w,
                             int32_t x, int32_t y, uint32_t width, uint32_t height) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    win->x = x;
    win->y = y;
    win->width = width;
    win->height = height;
    win->mapped = true;
    check_collisions(ctx);
}

static void handle_title(void *data, struct org_kde_plasma_window *w, const char *title) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    if (title) {
        strncpy(win->title, title, sizeof(win->title) - 1);
    }
    check_collisions(ctx);
}

static void handle_app_id(void *data, struct org_kde_plasma_window *w, const char *app_id) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    if (app_id) {
        strncpy(win->app_id, app_id, sizeof(win->app_id) - 1);
    }
    check_collisions(ctx);
}

static void handle_state(void *data, struct org_kde_plasma_window *w, uint32_t flags) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    win->state_flags = flags;
    win->minimized = (flags & ORG_KDE_PLASMA_WINDOW_MANAGEMENT_STATE_MINIMIZED) != 0;
    win->fullscreen = (flags & ORG_KDE_PLASMA_WINDOW_MANAGEMENT_STATE_FULLSCREEN) != 0;
    win->on_all_desktops = (flags & ORG_KDE_PLASMA_WINDOW_MANAGEMENT_STATE_ON_ALL_DESKTOPS) != 0;
    check_collisions(ctx);
}

static void handle_pid(void *data, struct org_kde_plasma_window *w, uint32_t pid) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    win->pid = pid;
    check_collisions(ctx);
}

static void handle_unmapped(void *data, struct org_kde_plasma_window *w) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;

    org_kde_plasma_window_destroy(w);
    win->win = NULL;
    win->in_use = false;
    check_collisions(ctx);
}

static void handle_virtual_desktop_entered(void *data, struct org_kde_plasma_window *w, const char *id) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    add_window_desktop(win, id);
    check_collisions(ctx);
}

static void handle_virtual_desktop_left(void *data, struct org_kde_plasma_window *w, const char *id) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct WinEntry *win = find_window(ctx, w);
    if (!win) return;
    remove_window_desktop(win, id);
    check_collisions(ctx);
}

static void handle_dummy(void *data, struct org_kde_plasma_window *w) { (void)data; (void)w; }
static void handle_dummy_p(void *data, struct org_kde_plasma_window *w, struct org_kde_plasma_window *p) { (void)data; (void)w; (void)p; }
static void handle_dummy_s(void *data, struct org_kde_plasma_window *w, const char *s) { (void)data; (void)w; (void)s; }
static void handle_dummy_ss(void *data, struct org_kde_plasma_window *w, const char *s1, const char *s2) { (void)data; (void)w; (void)s1; (void)s2; }
static void handle_dummy_i(void *data, struct org_kde_plasma_window *w, int32_t i) { (void)data; (void)w; (void)i; }

static const struct org_kde_plasma_window_listener s_window_listener = {
    .title_changed = handle_title,
    .app_id_changed = handle_app_id,
    .state_changed = handle_state,
    .virtual_desktop_changed = handle_dummy_i,
    .themed_icon_name_changed = handle_dummy_s,
    .unmapped = handle_unmapped,
    .initial_state = handle_dummy,
    .parent_window = handle_dummy_p,
    .geometry = handle_geometry,
    .icon_changed = handle_dummy,
    .pid_changed = handle_pid,
    .virtual_desktop_entered = handle_virtual_desktop_entered,
    .virtual_desktop_left = handle_virtual_desktop_left,
    .application_menu = handle_dummy_ss,
    .activity_entered = handle_dummy_s,
    .activity_left = handle_dummy_s,
    .resource_name_changed = handle_dummy_s,
};

static void handle_window(void *data, struct org_kde_plasma_window_management *wm, uint32_t id) {
    (void)data; (void)wm; (void)id;
}

static void handle_window_with_uuid(void *data, struct org_kde_plasma_window_management *wm,
                                    uint32_t id, const char *uuid) {
    (void)id;
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    if (!ctx || !ctx->wm || !uuid) return;

    for (int i = 0; i < MAX_WINDOWS; ++i) {
        if (ctx->windows[i].in_use && strcmp(ctx->windows[i].uuid, uuid) == 0) {
            return;
        }
    }

    struct org_kde_plasma_window *win = org_kde_plasma_window_management_get_window_by_uuid(wm, uuid);
    if (!win) return;

    for (int i = 0; i < MAX_WINDOWS; ++i) {
        if (!ctx->windows[i].in_use) {
            ctx->windows[i].in_use = true;
            ctx->windows[i].win = win;
            strncpy(ctx->windows[i].uuid, uuid, sizeof(ctx->windows[i].uuid) - 1);
            ctx->windows[i].mapped = false;
            ctx->windows[i].minimized = false;
            ctx->windows[i].fullscreen = false;
            ctx->windows[i].state_flags = 0;
            ctx->windows[i].x = 0;
            ctx->windows[i].y = 0;
            ctx->windows[i].width = 0;
            ctx->windows[i].height = 0;
            ctx->windows[i].pid = 0;
            ctx->windows[i].desktop_count = 0;
            ctx->windows[i].on_all_desktops = false;
            ctx->windows[i].desktop_overflow = false;
            memset(ctx->windows[i].desktops, 0, sizeof(ctx->windows[i].desktops));
            ctx->windows[i].app_id[0] = '\0';
            ctx->windows[i].title[0] = '\0';

            org_kde_plasma_window_add_listener(win, &s_window_listener, ctx);
            return;
        }
    }
}

static void handle_vd_desktop_id(void *data, struct org_kde_plasma_virtual_desktop *desktop, const char *id) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct DesktopEntry *entry = find_desktop(ctx, desktop);
    if (!entry || !id || strlen(id) >= DESKTOP_ID_LEN) return;
    snprintf(entry->id, DESKTOP_ID_LEN, "%s", id);
}

static void handle_vd_name(void *data, struct org_kde_plasma_virtual_desktop *desktop, const char *name) {
    (void)data; (void)desktop; (void)name;
}

static void handle_vd_activated(void *data, struct org_kde_plasma_virtual_desktop *desktop) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct DesktopEntry *entry = find_desktop(ctx, desktop);
    if (!entry) return;
    entry->active = true;
    check_collisions(ctx);
}

static void handle_vd_deactivated(void *data, struct org_kde_plasma_virtual_desktop *desktop) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct DesktopEntry *entry = find_desktop(ctx, desktop);
    if (!entry) return;
    entry->active = false;
    check_collisions(ctx);
}

static void handle_vd_done(void *data, struct org_kde_plasma_virtual_desktop *desktop) {
    (void)desktop;
    check_collisions((struct DodgeContext *)data);
}

static void handle_vd_removed(void *data, struct org_kde_plasma_virtual_desktop *desktop) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct DesktopEntry *entry = find_desktop(ctx, desktop);
    if (!entry) return;
    entry->active = false;
    entry->in_use = false;
    entry->desktop = NULL;
    org_kde_plasma_virtual_desktop_destroy(desktop);
    check_collisions(ctx);
}

static const struct org_kde_plasma_virtual_desktop_listener s_vd_listener = {
    .desktop_id = handle_vd_desktop_id,
    .name = handle_vd_name,
    .activated = handle_vd_activated,
    .deactivated = handle_vd_deactivated,
    .done = handle_vd_done,
    .removed = handle_vd_removed,
};

static void handle_vdm_desktop_created(void *data, struct org_kde_plasma_virtual_desktop_management *vdm,
                                       const char *desktop_id, uint32_t position) {
    (void)vdm; (void)position;
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    if (!ctx || !desktop_id || strlen(desktop_id) >= DESKTOP_ID_LEN) {
        if (ctx) ctx->desktop_overflow = true;
        return;
    }
    if (find_desktop_by_id(ctx, desktop_id)) return;

    for (int i = 0; i < MAX_DESKTOPS; ++i) {
        if (!ctx->desktops[i].in_use) {
            ctx->desktops[i].in_use = true;
            ctx->desktops[i].active = false;
            snprintf(ctx->desktops[i].id, DESKTOP_ID_LEN, "%s", desktop_id);
            ctx->desktops[i].desktop =
                org_kde_plasma_virtual_desktop_management_get_virtual_desktop(ctx->vdm, desktop_id);
            if (ctx->desktops[i].desktop) {
                org_kde_plasma_virtual_desktop_add_listener(ctx->desktops[i].desktop, &s_vd_listener, ctx);
            } else {
                ctx->desktops[i].in_use = false;
            }
            return;
        }
    }
    ctx->desktop_overflow = true;
}

static void handle_vdm_desktop_removed(void *data, struct org_kde_plasma_virtual_desktop_management *vdm,
                                       const char *desktop_id) {
    (void)vdm;
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    struct DesktopEntry *entry = find_desktop_by_id(ctx, desktop_id);
    if (!entry) return;
    entry->active = false;
    entry->in_use = false;
    if (entry->desktop) {
        org_kde_plasma_virtual_desktop_destroy(entry->desktop);
        entry->desktop = NULL;
    }
    check_collisions(ctx);
}

static void handle_vdm_done(void *data, struct org_kde_plasma_virtual_desktop_management *vdm) {
    (void)vdm;
    check_collisions((struct DodgeContext *)data);
}

static void handle_vdm_rows(void *data, struct org_kde_plasma_virtual_desktop_management *vdm, uint32_t rows) {
    (void)data; (void)vdm; (void)rows;
}

static const struct org_kde_plasma_virtual_desktop_management_listener s_vdm_listener = {
    .desktop_created = handle_vdm_desktop_created,
    .desktop_removed = handle_vdm_desktop_removed,
    .done = handle_vdm_done,
    .rows = handle_vdm_rows,
};

static void handle_show_desktop(void *data, struct org_kde_plasma_window_management *wm, uint32_t state) {
    (void)wm;
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    if (state != 0) {
        if (ctx->last_overlap) {
            ctx->last_overlap = false;
            if (ctx->callback) ctx->callback(false, ctx->user_data);
        }
    } else {
        check_collisions(ctx);
    }
}

static void handle_stacking(void *data, struct org_kde_plasma_window_management *wm, struct wl_array *ids) {
    (void)data; (void)wm; (void)ids;
}

static void handle_stacking_uuid(void *data, struct org_kde_plasma_window_management *wm, const char *uuids) {
    (void)data; (void)wm; (void)uuids;
}

static const struct org_kde_plasma_window_management_listener s_wm_listener = {
    .show_desktop_changed = handle_show_desktop,
    .window = handle_window,
    .stacking_order_changed = handle_stacking,
    .stacking_order_uuid_changed = handle_stacking_uuid,
    .window_with_uuid = handle_window_with_uuid,
};

static void registry_global(void *data, struct wl_registry *registry, uint32_t name,
                            const char *interface, uint32_t version) {
    struct DodgeContext *ctx = (struct DodgeContext *)data;
    if (strcmp(interface, "org_kde_plasma_window_management") == 0) {
        uint32_t bind_ver = version > 16 ? 16 : version;
        ctx->wm = (struct org_kde_plasma_window_management *)wl_registry_bind(
            registry, name, &org_kde_plasma_window_management_interface, bind_ver);
        org_kde_plasma_window_management_add_listener(ctx->wm, &s_wm_listener, ctx);
    } else if (strcmp(interface, "org_kde_plasma_virtual_desktop_management") == 0) {
        uint32_t bind_ver = version > 2 ? 2 : version;
        ctx->vdm = (struct org_kde_plasma_virtual_desktop_management *)wl_registry_bind(
            registry, name, &org_kde_plasma_virtual_desktop_management_interface, bind_ver);
        org_kde_plasma_virtual_desktop_management_add_listener(ctx->vdm, &s_vdm_listener, ctx);
    }
}

static void registry_global_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}

static const struct wl_registry_listener s_reg_listener = {
    .global = registry_global,
    .global_remove = registry_global_remove,
};

struct DodgeContext *dodge_create(int32_t notch_x, int32_t notch_y, uint32_t notch_w, uint32_t notch_h,
                                  DodgeOverlapCallback callback, void *user_data) {
    struct DodgeContext *ctx = (struct DodgeContext *)calloc(1, sizeof(struct DodgeContext));
    if (!ctx) return NULL;

    ctx->notch_x = notch_x;
    ctx->notch_y = notch_y;
    ctx->notch_w = notch_w;
    ctx->notch_h = notch_h;
    ctx->callback = callback;
    ctx->user_data = user_data;
    ctx->my_pid = (uint32_t)getpid();
    atomic_init(&ctx->running, true);
    pipe2(ctx->wake_pipe, O_NONBLOCK | O_CLOEXEC);

    ctx->display = wl_display_connect(NULL);
    if (!ctx->display) {
        close(ctx->wake_pipe[0]);
        close(ctx->wake_pipe[1]);
        free(ctx);
        return NULL;
    }

    ctx->registry = wl_display_get_registry(ctx->display);
    wl_registry_add_listener(ctx->registry, &s_reg_listener, ctx);
    wl_display_roundtrip(ctx->display);
    wl_display_roundtrip(ctx->display);

    return ctx;
}

void dodge_update_notch(struct DodgeContext *ctx, int32_t notch_x, int32_t notch_y, uint32_t notch_w, uint32_t notch_h) {
    if (!ctx) return;
    ctx->notch_x = notch_x;
    ctx->notch_y = notch_y;
    ctx->notch_w = notch_w;
    ctx->notch_h = notch_h;
    check_collisions(ctx);
}

void dodge_run(struct DodgeContext *ctx) {
    if (!ctx || !ctx->display) return;

    int wl_fd = wl_display_get_fd(ctx->display);
    int wake_fd = ctx->wake_pipe[0];

    while (atomic_load(&ctx->running)) {
        while (wl_display_prepare_read(ctx->display) != 0) {
            wl_display_dispatch_pending(ctx->display);
        }
        wl_display_flush(ctx->display);

        struct pollfd fds[2];
        fds[0].fd = wl_fd;
        fds[0].events = POLLIN;
        fds[0].revents = 0;

        fds[1].fd = wake_fd;
        fds[1].events = POLLIN;
        fds[1].revents = 0;

        int ret = poll(fds, 2, 250);
        if (ret < 0) {
            wl_display_cancel_read(ctx->display);
            if (errno == EINTR) continue;
            break;
        }

        if (fds[1].revents & POLLIN) {
            wl_display_cancel_read(ctx->display);
            break;
        }

        if (fds[0].revents & POLLIN) {
            if (wl_display_read_events(ctx->display) < 0) {
                break;
            }
            wl_display_dispatch_pending(ctx->display);
        } else {
            wl_display_cancel_read(ctx->display);
        }
    }
}

void dodge_stop(struct DodgeContext *ctx) {
    if (!ctx) return;
    atomic_store(&ctx->running, false);
    char b = 1;
    if (ctx->wake_pipe[1] >= 0) {
        (void)write(ctx->wake_pipe[1], &b, 1);
    }
}

void dodge_destroy(struct DodgeContext *ctx) {
    if (!ctx) return;
    ctx->callback = NULL;

    for (int i = 0; i < MAX_WINDOWS; ++i) {
        if (ctx->windows[i].in_use && ctx->windows[i].win) {
            org_kde_plasma_window_destroy(ctx->windows[i].win);
            ctx->windows[i].win = NULL;
            ctx->windows[i].in_use = false;
        }
    }

    for (int i = 0; i < MAX_DESKTOPS; ++i) {
        if (ctx->desktops[i].in_use && ctx->desktops[i].desktop) {
            org_kde_plasma_virtual_desktop_destroy(ctx->desktops[i].desktop);
            ctx->desktops[i].desktop = NULL;
            ctx->desktops[i].in_use = false;
        }
    }

    if (ctx->vdm) {
        org_kde_plasma_virtual_desktop_management_destroy(ctx->vdm);
        ctx->vdm = NULL;
    }
    if (ctx->wm) {
        org_kde_plasma_window_management_destroy(ctx->wm);
        ctx->wm = NULL;
    }
    if (ctx->registry) {
        wl_registry_destroy(ctx->registry);
        ctx->registry = NULL;
    }
    if (ctx->display) {
        wl_display_flush(ctx->display);
        wl_display_disconnect(ctx->display);
        ctx->display = NULL;
    }
    if (ctx->wake_pipe[0] >= 0) close(ctx->wake_pipe[0]);
    if (ctx->wake_pipe[1] >= 0) close(ctx->wake_pipe[1]);

    free(ctx);
}
