#include "upatch-resolve.h"

#include <stdlib.h>
#include <stddef.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>

#include "list.h"
#include "upatch-meta.h"

// static int list_add_symbol(struct list_head *head, patch_symbols_t *sym)
// {
// 	patch_symbols_t *newsym = (patch_symbols_t *)malloc(sizeof(patch_symbols_t));
// 	if (newsym == NULL)
// 		return -ENOMEM;

// 	memset(newsym, 0, sizeof(patch_symbols_t));
// 	strncpy(newsym->name, sym->name, sizeof(newsym->name));
// 	newsym->offset = sym->offset;
// 	INIT_LIST_HEAD(&newsym->self);
// 	list_add(&newsym->self, head);
// 	return 0;
// }

struct list_head* patch_symbols_resolve(const char *target_elf, const char *patch_file) {
    // Example code to add symbols:
    // struct list_head *head = malloc(sizeof(struct list_head));
    // INIT_LIST_HEAD(head);

    // for (int i = 0; i < 10; i++) {
    //     patch_symbols_t *sym = malloc(sizeof(patch_symbols_t));
    //     sprintf(sym->name, "sym_%d", i);
    //     sym->offset = i;
    //     list_add_symbol(head, sym);
    // }

    // return head;
}

void patch_symbols_free(struct list_head *symbols) {

}
