#ifndef DODGE_NATIVE_H
#define DODGE_NATIVE_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*DodgeOverlapCallback)(bool overlapping, void *user_data);

struct DodgeContext;

struct DodgeContext *dodge_create(int32_t notch_x, int32_t notch_y, uint32_t notch_w, uint32_t notch_h,
                                  DodgeOverlapCallback callback, void *user_data);
void dodge_update_notch(struct DodgeContext *ctx, int32_t notch_x, int32_t notch_y, uint32_t notch_w, uint32_t notch_h);
void dodge_run(struct DodgeContext *ctx);
void dodge_stop(struct DodgeContext *ctx);
void dodge_destroy(struct DodgeContext *ctx);

#ifdef __cplusplus
}
#endif

#endif // DODGE_NATIVE_H
