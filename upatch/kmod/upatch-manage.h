// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_MANAGE_H
#define _UPATCH_MANAGE_H

enum upatch_module_state {
    UPATCH_STATE_REMOVED = 0x1, /* Original status - No patch */
	UPATCH_STATE_ATTACHED,  /* Attach patch to the binary */
    UPATCH_STATE_RESOLVED, /* Resolve the patch */
    UPATCH_STATE_ACTIVED, /* Activate the patch */
};

#endif /* _UPATCH_MANAGE_H */
