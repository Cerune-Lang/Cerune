
    /* Copy packed ABI inputs before prepare. No source argument is evaluated twice. */
    uint64_t argc=(uint64_t)@ARGS@.length, length=(uint64_t)@STDIN@.length;
    if (argc > SIZE_MAX/sizeof(cerune_process_bytes) || argc > PTRDIFF_MAX/sizeof(cerune_process_bytes) || length > PTRDIFF_MAX)
        cerune_runtime_fail("allocation-size-overflow",origin);
    cerune_process_bytes *arguments=argc ? malloc((size_t)argc*sizeof(*arguments)) : NULL;
    unsigned char *input=length ? malloc((size_t)length) : NULL;
    if ((argc && !arguments) || (length && !input)) {
        free(arguments); free(input); cerune_runtime_fail("allocation-failed",origin);
    }
    for (uint64_t i=0;i<argc;++i) {
        cerune_string v=((cerune_string *)@ARGS@.owner->data)[i];
        arguments[i]=(cerune_process_bytes){v.data,v.length};
    }
    for (uint64_t i=0;i<length;++i) input[i]=(unsigned char)((int64_t *)@STDIN@.owner->data)[i];
    cerune_process_request_v1 value = {1,sizeof(value),
        {@TARGET@.data,@TARGET@.length},arguments,argc,{input,length},
        @OUT_LIMIT@,@ERR_LIMIT@,@TIME@,node,source,start,end};
    uint32_t status=cerune_process_v1_prepare(cerune_process_context_value,&value);
    free(arguments); free(input);
    if (status==1) {
        /* Both language arrays are reserved before any clock read or launch. */
        int reservation=cerune_process_reserve(@OUT_LIMIT@,@ERR_LIMIT@,&@OUT@,&@ERR@);
        status=cerune_process_v1_execute(cerune_process_context_value,(uint64_t)reservation);
    }
    cerune_process_check(status,2,origin);
    uint64_t w[32];
    cerune_process_check(cerune_process_v1_report(cerune_process_context_value,w,32),0,origin);
    if (w[0]!=1 || w[1]!=32 || w[28] || w[29] || w[30] || w[31] ||
        w[14]>(uint64_t)@OUT@.length || w[15]>(uint64_t)@ERR@.length)
        cerune_process_contract(13,origin);
    cerune_process_finalize(&@OUT@,w[14],1,origin);
    cerune_process_finalize(&@ERR@,w[15],2,origin);
