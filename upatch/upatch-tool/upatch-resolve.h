#ifndef __UPATCH_RESOLVE_H_
#define __UPATCH_RESOLVE_H_

#include "list.h"
#include "upatch-elf.h"

struct list_head* patch_symbols_resolve(struct upatch_elf *uelf, struct running_elf *relf);
void patch_symbols_free(struct list_head *symbols);

#endif
