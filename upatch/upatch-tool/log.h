#ifndef __UPATCH_LOG_
#define __UPATCH_LOG_

#include <stdio.h>

#define log(level, format, ...) \
({ \
	printf(""format"\n", ##__VA_ARGS__); \
})

#define log_debug(format, ...) log(DEBUG, format, ##__VA_ARGS__)
#define log_normal(format, ...) log(NORMAL, format, ##__VA_ARGS__)
#define log_warn(format, ...) log(WARN, format, ##__VA_ARGS__)

#endif
