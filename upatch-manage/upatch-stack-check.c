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
        struct upatch_func_addr addr = upatch_func->addr;

        if (action == ACTIVE) {
            start = addr.old_addr;
            end = addr.old_addr + addr.old_size;
        } else if (action == DEACTIVE) {
            start = addr.new_addr;
            end = addr.new_addr + addr.new_size;
        } else {
            log_error("Unknown upatch action\n");
            return -1;
        }
        if (pc >= start && pc <= end) {
            log_error("Failed to check stack, running function: %s\n",
                upatch_func->name);
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
    while (1) {
        if (stack_check(uinfo, (unsigned long)pc, action) < 0) {
            return -1;
        }
        pc = ptrace(PTRACE_PEEKDATA, pid, sp, NULL);
        if (pc == -1 && errno != 0) {
            break;
        }
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
            return -EBUSY;
        }
    }
    return 0;
}