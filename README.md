# CShep: A command shepherd with reference counting

Often in Unix there are resources that can be allocated with one command and deallocated with another. The quintessential example of this is `mount` and `umount`.

If multiple processes want access to the same resource, however, there may be contention over running the allocation and deallocation commands. This is where CShep comes in. CShep provides a dead-simple socket interface to manage the execution of allocation and deallocation commands via reference counting.

## Invocation

```bash
# these three arguments are required
cshep --socket cshep.sock --start "touch file" --end "rm file"
# `--check-success` is optional
cshep --socket cshep.sock --start "ok_to_fail start" --end "ok_to_fail end" --check-success false
```

## Socket Interface

Following are the available commands over the socket. Each command is a single ASCII character. Unknown commands are ignored. Due to the genius of Unicode, Unicode input will not be harmful and will simply be ignored.

| Command Character | Action                                                           | Output                                                   |
| ----------------- | ---------------------------------------------------------------- | -------------------------------------------------------- |
| `+`               | Increment the reference count.                                   | None                                                     |
| `-`               | Decrement the reference count.                                   | None                                                     |
| `c`               | Get the current reference count.                                 | Reference count as an ASCII number followed by a newline |
| `q`               | Quit if possible. Does not quit if outstanding references exist. | None                                                     |
| `Q`               | Force quit regardless of outstanding references.                 | None                                                     |
