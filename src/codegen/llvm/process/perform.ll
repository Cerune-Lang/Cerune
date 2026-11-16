define internal { %cerune.array, %cerune.array } @cerune.process.perform(%cerune.string %target, %cerune.array %arguments, %cerune.array %input, i64 %out.limit, i64 %err.limit, i64 %time, i64 %node, i64 %source, i64 %start, i64 %end, ptr %words, ptr %origin, ptr %failure) {
entry:
  %out = alloca %cerune.array
  %err = alloca %cerune.array
  %request = alloca %cerune.process.request
  store %cerune.array zeroinitializer, ptr %out
  store %cerune.array zeroinitializer, ptr %err
  %target.data = extractvalue %cerune.string %target, 0
  %target.len = extractvalue %cerune.string %target, 1
  %input.len = extractvalue %cerune.array %input, 1
  %input.data = call ptr @cerune.process.pack(%cerune.array %input, ptr %failure)
  ; Strings and ABI Bytes both contain { ptr, i64 }; prepare copies these views.
  %args.data = call ptr @cerune.process.array.data(%cerune.array %arguments)
  %args.len = extractvalue %cerune.array %arguments, 1
  %v0 = insertvalue %cerune.process.request zeroinitializer, i64 1, 0
  %v1 = insertvalue %cerune.process.request %v0, i64 120, 1
  %v2 = insertvalue %cerune.process.request %v1, ptr %target.data, 2, 0
  %v3 = insertvalue %cerune.process.request %v2, i64 %target.len, 2, 1
  %v4 = insertvalue %cerune.process.request %v3, ptr %args.data, 3
  %v5 = insertvalue %cerune.process.request %v4, i64 %args.len, 4
  %v6 = insertvalue %cerune.process.request %v5, ptr %input.data, 5, 0
  %v7 = insertvalue %cerune.process.request %v6, i64 %input.len, 5, 1
  %v8 = insertvalue %cerune.process.request %v7, i64 %out.limit, 6
  %v9 = insertvalue %cerune.process.request %v8, i64 %err.limit, 7
  %v10 = insertvalue %cerune.process.request %v9, i64 %time, 8
  %v11 = insertvalue %cerune.process.request %v10, i64 %node, 9
  %v12 = insertvalue %cerune.process.request %v11, i64 %source, 10
  %v13 = insertvalue %cerune.process.request %v12, i64 %start, 11
  %v14 = insertvalue %cerune.process.request %v13, i64 %end, 12
  store %cerune.process.request %v14, ptr %request
  %context = load ptr, ptr @cerune.process.context
  %prepared = call i32 @cerune_process_v1_prepare(ptr %context, ptr %request)
  call void @free(ptr %input.data)
  %pending = icmp eq i32 %prepared, 1
  br i1 %pending, label %reserve, label %ready
reserve:
  %reservation = call i64 @cerune.process.reserve(i64 %out.limit, i64 %err.limit, ptr %out, ptr %err)
  %executed = call i32 @cerune_process_v1_execute(ptr %context, i64 %reservation)
  br label %ready
ready:
  %status = phi i32 [%prepared, %entry], [%executed, %reserve]
  call void @cerune.process.check(i32 %status, i32 2, ptr %origin)
  %reported = call i32 @cerune_process_v1_report(ptr %context, ptr %words, i64 32)
  call void @cerune.process.check(i32 %reported, i32 0, ptr %origin)
  %version = load i64, ptr %words
  %size.p = getelementptr i64, ptr %words, i64 1
  %size = load i64, ptr %size.p
  %version.ok = icmp eq i64 %version, 1
  %size.ok = icmp eq i64 %size, 32
  %header.ok = and i1 %version.ok, %size.ok
  %header.status = select i1 %header.ok, i32 0, i32 13
  call void @cerune.process.check(i32 %header.status, i32 0, ptr %origin)
  br label %reserved
reserved:
  %i = phi i64 [28, %ready], [%next, %reserved]
  %p = getelementptr i64, ptr %words, i64 %i
  %word = load i64, ptr %p
  %zero = icmp eq i64 %word, 0
  %reserved.status = select i1 %zero, i32 0, i32 13
  call void @cerune.process.check(i32 %reserved.status, i32 0, ptr %origin)
  %next = add i64 %i, 1
  %finished = icmp eq i64 %next, 32
  br i1 %finished, label %finalize, label %reserved
finalize:
  %nout.p = getelementptr i64, ptr %words, i64 14
  %nerr.p = getelementptr i64, ptr %words, i64 15
  %nout = load i64, ptr %nout.p
  %nerr = load i64, ptr %nerr.p
  call void @cerune.process.finalize(ptr %out, i64 %nout, i64 1, ptr %origin)
  call void @cerune.process.finalize(ptr %err, i64 %nerr, i64 2, ptr %origin)
  %out.value = load %cerune.array, ptr %out
  %err.value = load %cerune.array, ptr %err
  %a = insertvalue { %cerune.array, %cerune.array } poison, %cerune.array %out.value, 0
  %b = insertvalue { %cerune.array, %cerune.array } %a, %cerune.array %err.value, 1
  ret { %cerune.array, %cerune.array } %b
}
