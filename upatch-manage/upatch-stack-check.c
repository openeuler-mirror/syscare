#include <errno.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <sys/socket.h>
#include <sys/resource.h>
#include <sys/time.h>

#include "upatch-elf.h"
#include "upatch-stack-check.h"
#include "upatch-ptrace.h"
#include "upatch-common.h"
#include "log.h"

static int stack_check(struct upatch_info *uinfo, unsigned long pc, upatch_action_t action)
{
    unsigned long start;
    unsigned long end;

    for (size_t i = 0; i < uinfo->changed_func_num; i++) {
        struct upatch_func_addr addr = uinfo->funcs[i].addr;

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
                uinfo->funcs[i].name);
            return -1;
        }
    }
    return 0;
}

static unsigned long *stack_alloc(size_t *size)
{
    struct rlimit rl;
    unsigned long *stack = NULL;

    if (getrlimit(RLIMIT_STACK, &rl) != 0) {
        log_error("Failed to get system stack size config\n");
        return 0;
    }

    *size = rl.rlim_cur;
    stack = (unsigned long *)malloc(*size);
    if (stack == NULL) {
        log_error("Failed to malloc stack\n");
    }

    return stack;
}

static size_t read_stack(struct upatch_process *proc,
    unsigned long *stack, size_t size, unsigned long sp)
{
    return (size_t)pread(proc->memfd, (void *)stack, size, (off_t)sp);
}

static int stack_check_each_pid(struct upatch_process *proc,
    struct upatch_info *uinfo, int pid, upatch_action_t action)
{
    unsigned long sp, pc;
    unsigned long *stack = NULL;
    size_t stack_size = 0;
    int ret = 0;

    if (upatch_arch_reg_init(pid, &sp, &pc) < 0) {
        return -1;
    }
    ret = stack_check(uinfo, pc, action);
    if (ret < 0) {
        return ret;
    }

    stack = stack_alloc(&stack_size);
    if (stack == NULL) {
        return -1;
    }

    stack_size = read_stack(proc, stack, stack_size, sp);
    log_debug("[%d] Stack size %lu, region [0x%lx, 0x%lx]\n",
        pid, stack_size, sp, sp + stack_size);

    for (size_t i = 0; i < stack_size / sizeof(*stack); i++) {
        if (stack[i] == 0 || stack[i] == -1UL) {
            continue;
        }

        ret = stack_check(uinfo, stack[i], action);
        if (ret < 0) {
            goto free;
        }
    }
free:
    free(stack);
    return ret;
}

int upatch_stack_check(struct upatch_info *uinfo, struct upatch_process *proc,
    upatch_action_t action)
{
    struct upatch_ptrace_ctx *pctx;
    struct timeval start, end;

    if (gettimeofday(&start, NULL) < 0) {
        log_error("Failed to get stack check start time\n");
    }

    list_for_each_entry(pctx, &proc->ptrace.pctxs, list) {
        if (stack_check_each_pid(proc, uinfo, pctx->pid, action) < 0) {
            return -EBUSY;
        }
    }

    if (gettimeofday(&end, NULL) < 0) {
        log_error("Failed to get stack check end time\n");
    }

    log_debug("Stack check time %ld microseconds\n",
        get_microseconds(&start, &end));
    return 0;
}
