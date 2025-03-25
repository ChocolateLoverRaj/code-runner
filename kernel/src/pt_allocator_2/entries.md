# Calculating virt and phys entries
`phys_entries` - found by looking at Limine memory map response
`virt_entries` - found by traversing Cr3

`up_to_total_meta_pages_used` = (`phys_entries` + `virt_entries`) / `ENTRIES_PER_PAGE`;
`up_to_n_meta_phys_entries` = `up_to_total_meta_pages_used` * 4;
`up_to_n_meta_virt_entries` = `up_to_total_meta_pages_used` * 1;

`up_to_total_meta_2_pages_used` = (`up_to_n_meta_phys_entries` + `up_to_n_meta_virt_entries`) / `ENTRIES_PER_PAGE`;
= (`up_to_total_meta_pages_used` * 4 + `up_to_total_meta_pages_used` * 1) / `ENTRIES_PER_PAGE`
= (`up_to_total_meta_pages_used` * 5) / `ENTRIES_PER_PAGE`
`up_to_n_meta_2_phys_entries` = `up_to_total_meta_2_pages_used` * 4;
`up_to_n_meta_2_virt_entries` = `up_to_total_meta_2_pages_used` * 1;

`up_to_total_meta_3_pages_used` = (`up_to_n_meta_2_phys_entries` + `up_to_n_meta_2_virt_entries`) / `ENTRIES_PER_PAGE`;
= (`up_to_total_meta_2_pages_used` * 4 + `up_to_total_meta_2_pages_used` * 1) / `ENTRIES_PER_PAGE`
= (`up_to_total_meta_2_pages_used` * 5) / `ENTRIES_PER_PAGE`
= (`up_to_n_meta_phys_entries` + `up_to_n_meta_virt_entries`) * 5 / `ENTRIES_PER_PAGE` ^ 2

This becomes a sum of an infinite series - https://www.geeksforgeeks.org/sum-of-infinite-series-formula/
With the forumula `a` / (1 - `r`)
In this case
`a` = (`phys_entries` + `virt_entries`) / `ENTRIES_PER_PAGE`
`r` = 5 / `ENTRIES_PER_PAGE`

So
`up_to_recursive_meta_pages_used` = (`phys_entries` + `virt_entries`) / `ENTRIES_PER_PAGE` / (1 - 5 / `ENTRIES_PER_PAGE`)

Re-arranging to work with integer division:
`up_to_recursive_meta_pages_used` = (`phys_entries` + `virt_entries`) / (`ENTRIES_PER_PAGE` - 5)
and then
`up_to_recursive_meta_phys_entries` = `up_to_recursive_meta_pages_used` * 4;
`up_to_recursive_meta_virt_entries` = `up_to_recursive_meta_pages_used` * 1;

And then
`phys_entries_to_reserve` = `phys_entries` + `up_to_recursive_meta_phys_entries`
`virt_entries_to_reserve` = `virt_entries` + `up_to_recursive_meta_virt_entries`

And let's use div_ceil to be sure
