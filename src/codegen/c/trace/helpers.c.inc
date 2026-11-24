static int cerune_trace_branch(int taken, const char *yes, const char *no) {
    cerune_trace_record(taken ? yes : no);
    return taken;
}

/* runtime-v1 passes static code and origin strings, including file= for imported sources. */
static void cerune_trace_stop(const char *code, const char *origin) {
    if (cerune_trace_path) {
        size_t node, file = 0, start, end;
        if (sscanf(origin, " node=%zu file=%zu bytes=%zu..%zu", &node, &file, &start, &end) == 4 ||
            sscanf(origin, " node=%zu bytes=%zu..%zu", &node, &start, &end) == 3) {
            static char row[256];
            int n = snprintf(row, sizeof(row), ",\"event\":\"stop\",\"code\":\"%s\",\"node\":%zu,\"file\":%zu,\"start\":%zu,\"end\":%zu}\n",
                code, node, file, start, end);
            if (n > 0 && (size_t)n < sizeof(row)) cerune_trace_record(row);
        }
        cerune_trace_save();
    }
}
static void cerune_trace_exit(void) {
    cerune_trace_save();
    _Exit(1);
}
