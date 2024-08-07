#include <errno.h>
#include <sys/wait.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/socket.h>

#include "upatch-elf.h"
#include "upatch-process.h"
#include "upatch-stack-check.h"
#include "upatch-ptrace.h"
#include "log.h"

static int stack_check(struct upatch_info *uinfo, unsigned long pc, upatch_action_t action)
{
    unsigned long start, end;

    for (size_t i = 0; i < uinfo->changed_func_num; i++) {
        struct upatch_info_func *upatch_func = &uinfo->funcs[i];
        if (action == ACTIVE) {
            start = upatch_func->old_addr;
            end = upatch_func->old_addr + upatch_func->old_size;
        } else if (action == DEACTIVE) {
            start = upatch_func->new_addr;
            end = upatch_func->new_addr + upatch_func->new_size;
        } else {
            log_error("Unknown upatch action\n");
            return -1;
        }
        if (pc >= start && pc <= end) {
            log_error("Stack check failed 0x%lx is running [0x%lx: 0x%lx]\n",
                pc, start, end);
                return -1;
        }
    }
    return 0;
}

static int stack_check_each_pid(struct upatch_info *uinfo, int pid, upatch_action_t action)
{
    long sp, pc;

    if (upatch_arch_unwind_init(pid, &sp, &pc) < 0) {
        return -1;
    }
    log_debug("Stack line:\n");
    while (1) {
        if (stack_check(uinfo, (unsigned long)pc, action) < 0) {
            return -1;
        }
        pc = ptrace(PTRACE_PEEKDATA, pid, sp, NULL);
        if (pc == -1 && errno != 0) {
            break;
        }
        log_debug("\t0x%lx: 0x%lx\n", sp, pc);
        sp += 8;
    }
    return 0;
}

int upatch_stack_check(struct upatch_info *uinfo,
    struct upatch_process *proc, upatch_action_t action)
{
    struct upatch_ptrace_ctx *pctx;

    list_for_each_entry(pctx, &proc->ptrace.pctxs, list) {
        if (stack_check_each_pid(uinfo, pctx->pid, action) < 0) {
            return -ERR_STACK_CHECK_FAILED;
        }
    }
    return 0;
}