#ifndef __UPATCH_RESOLVE_H_
#define __UPATCH_RESOLVE_H_

#include "list.h"

struct list_head* patch_symbols_resolve(const char *target_elf, const char *patch_file);
void patch_symbols_free(struct list_head *symbols);

#endif
