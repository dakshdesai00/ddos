.global flush_tlb_all

flush_tlb_all:
    dsb ish
    tlbi vmalle1is
    dsb ish
    isb
    ret
