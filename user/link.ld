/* A user program's layout.
 *
 * The kernel is at 0xffff_..., in TTBR1. A user program is at 0x0000_..., in TTBR0, and the
 * hardware picks between them from bits 63:48 of the address. So a user program can be linked
 * at a low address with no fear of colliding with anything: the kernel is not *in* its address
 * space in any sense it can observe. See notes/higher-half.md.
 *
 * PHDRS is explicit, and that is the point. Left to itself the linker will happily put .text
 * and .rodata in one PT_LOAD, sharing a page, and our loader refuses that (Error::SegmentsOverlap).
 * More importantly, being explicit is what lets us hand the kernel three segments with three
 * different permissions, so that W^X is a property of the file and not just of our good manners.
 */

ENTRY(_start)

PHDRS
{
    text   PT_LOAD FLAGS(5);   /* R+X  */
    rodata PT_LOAD FLAGS(4);   /* R    */
    data   PT_LOAD FLAGS(6);   /* R+W  */
}

SECTIONS
{
    . = 0x400000;

    .text : ALIGN(4096) {
        *(.text._start)         /* the entry point goes first, so `entry` is legible */
        *(.text .text.*)
    } :text

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    } :rodata

    .data : ALIGN(4096) {
        *(.data .data.*)

        /* The GOT, placed here on purpose and BEFORE .bss (milestone 161).
         *
         * Left unnamed, the linker appends orphan sections after the last one it was told about,
         * which is .bss. .got has file contents and .bss does not, so the `data` PT_LOAD's
         * p_filesz then stretches past .bss to cover it, p_memsz == p_filesz, and the segment has
         * no zero-fill tail at all. Two things break quietly: the loader has nothing to zero, so
         * .bss is whatever bytes happened to be in the file, and the kernel's own
         * `the_initrd_holds_a_native_executable` test loses the memsz > filesz it asserts on,
         * which is how this was found.
         *
         * It shows up on x86_64 and not on the other two. The GOT is not something this project
         * asked for: all three targets use the static relocation model and none of these programs
         * is position-independent, but LLVM's x86-64 backend still routes a few references through
         * a GOT entry, and the aarch64 and RISC-V backends produce none for the same source. So
         * this is one line that costs nothing where it is not needed and prevents a silent hole
         * where it is. */
        *(.got .got.plt)
    } :data

    /* .bss is in the `data` segment but occupies NO space in the file. That is what makes
     * p_memsz > p_filesz, and the loader has to zero the difference. See crates/elf.
     *
     * ALIGN(4096) here is not tidiness, it is the same rule the sections above follow, and it was
     * missing. A program with .bss and an EMPTY .data (nothing initialized and non-zero) gets its
     * .data collapsed by the linker, so the `data` PT_LOAD begins at .bss, which without this
     * alignment starts wherever .rodata happened to end. The two segments then share a page and the
     * loader refuses the program with SegmentsOverlap. Found by milestone 57's disk_surveyor, whose
     * 44 KiB of table buffers are all bss; every earlier program happened to have some .data. */
    .bss : ALIGN(4096) {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame*) }
}
