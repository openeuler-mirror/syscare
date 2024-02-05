// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_LOG_H
#define _UPATCH_HIJACKER_KO_LOG_H

#include <linux/module.h>
#include <linux/printk.h>

#ifdef pr_fmt
#undef pr_fmt
#endif

#define pr_fmt(fmt) "%s: " fmt, THIS_MODULE->name

#endif /* _UPATCH_HIJACKER_KO_LOG_H */
