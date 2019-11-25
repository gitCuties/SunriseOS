initSidebarItems({"constant":[["MAX_ELF_SIZE","Max size of an ELF before we issue a warning. Loader needs to keep its memory usage fairly low to avoid trouble, so we bail upon trying to load a file bigger than 128MiB."]],"fn":[["boot","Start the given titleid by loading its content from the provided filesystem."],["main",""]],"mod":[["elf_loader","Loads Elfs."]],"static":[["BOOT_FROM_FS",""],["CAPABILITIES",""],["HEADER",""],["PROCESSES",""],["PROCESS_STATE_CHANGED",""]],"struct":[["BOOT_FROM_FS","The filesystem to boot titles from."],["LoaderIface","Struct implementing the ldr:shel service."],["PROCESSES",""],["PROCESS_STATE_CHANGED","Public ReadableEvent that gets signaled when a process state changes. Other processes can get it by using the get_process_state_changed_event command."]]});