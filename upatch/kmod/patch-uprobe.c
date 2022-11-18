#include <linux/fs.h>
#include <linux/elf.h>
#include <linux/slab.h>

#include <asm/module.h>

elf_addr_t calculate_load_address(struct file *file, bool check_code)
{
    int ret, size, i;
    Elf_Ehdr elf_header;
    Elf_Phdr *phdr = NULL;
    elf_addr_t min_addr = -1;

    ret = kernel_read(file, &elf_header, sizeof(elf_header), 0);
    if (ret != sizeof(elf_header)) {
        pr_err("read elf header failed - %d \n", ret);
        goto out;
    }

    if (memcmp(elf_header.e_ident, ELFMAG, SELFMAG) != 0) {
        pr_err("provided path is not an ELF \n");
        goto out;
    }

    /* TODO: for ET_DYN, consider check PIE */
    if (elf_header.e_type != ET_EXEC && elf_header.e_type != ET_DYN) {
        pr_err("invalid elf type, it should be ET_EXEC or ET_DYN\n");
        goto out;
    }

    size = sizeof(Elf_Phdr) * elf_header.e_phnum;
    phdr = kmalloc(size, GFP_KERNEL);
    if (!phdr) {
        pr_err("kmalloc failed for load address calculate \n");
        goto out;
    }

    ret = kernel_read(file, phdr, size, &elf_header.e_phoff);
    if (ret < 0) {
        pr_err("kernel read failed - %d \n", ret);
        goto out;
    }

    for (i = 0; i < elf_header.e_phnum; i ++) {
        if (phdr[i].p_type != PT_LOAD)
            continue;
        if (!check_code ||
            (check_code && (phdr[i].p_flags & PF_X)))
            min_addr = min(min_addr, phdr[i].p_vaddr);
    }

out:
    if (phdr)
        kfree(phdr);
    return min_addr;
}