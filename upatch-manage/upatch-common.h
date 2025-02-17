// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-lib
 * Copyright (C) 2024 Huawei Technologies Co., Ltd.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#ifndef __UPATCH_COMMON__
#define __UPATCH_COMMON__

#include <stdbool.h>
#include <sys/time.h>

#define ALLOC_LINK(_new, _list) \
    do { \
        (_new) = calloc(1, sizeof(*(_new))); \
        if (!(_new)) { \
            ERROR("calloc"); \
        } \
        INIT_LIST_HEAD(&(_new)->list); \
        if (_list) { \
            list_add(&(_new)->list, (_list)); \
        } \
    } while (0)

static inline int page_shift(long n)
{
    int res = -1;

    while (n) {
        res++;
        n >>= 1;
    }

    return res;
}

#ifndef PAGE_SIZE
#define PAGE_SIZE sysconf(_SC_PAGE_SIZE)
#define PAGE_MASK (~(PAGE_SIZE - 1))
#define PAGE_SHIFT page_shift(PAGE_SIZE)
#endif

#define ARRAY_SIZE(x) (sizeof(x) / sizeof((x)[0]))
#define ALIGN(x, a) (((x) + (a) - 1) & (~((a) - 1)))
#define PAGE_ALIGN(x) ALIGN((x), (unsigned long)PAGE_SIZE)

#define ROUND_DOWN(x, m) ((x) & ~((m) - 1))
#define ROUND_UP(x, m) (((x) + (m) - 1) & ~((m) - 1))

#define BIT(x) (1UL << (x))

#define SEC2MICRO 1000000

static inline long get_microseconds(struct timeval *start, struct timeval *end)
{
    long sec = end->tv_sec - start->tv_sec;
    long usec = end->tv_usec - start->tv_usec;

    return sec * SEC2MICRO + usec;
}

bool streql(const char *, const char *);

#endif
