// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_UPROBE_H
#define _UPATCH_HIJACKER_KO_UPROBE_H

#include <linux/types.h>

struct uprobe_consumer;
struct pt_regs;

int handle_uprobe(struct uprobe_consumer *self, struct pt_regs *regs);

#endif /* _UPATCH_HIJACKER_KO_UPROBE_H */
