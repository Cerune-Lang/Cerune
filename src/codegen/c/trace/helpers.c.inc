static void cerune_trace_exit(void) {
    cerune_trace_save();
    _Exit(1);
}
