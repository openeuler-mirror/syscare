#ifndef __UPATCH_STACK_CHECK_H
#define __UPATCH_STACK_CHECK_H

#include "upatch-elf.h"
#include "upatch-process.h"

#define STACK_CHECK_RETRY_TIMES 3

typedef enum {
    ACTIVE,
    DEACTIVE,
} upatch_action_t;

int upatch_arch_unwind_init(int pid, long *sp, long *pc);
int upatch_stack_check(struct upatch_info *uinfo,
    struct upatch_process *proc, upatch_action_t action);
#endif