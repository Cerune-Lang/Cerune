#define _CRT_SECURE_NO_WARNINGS
#include "cerune_process_v1.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#define TARGET 1
#else
#include <unistd.h>
#define TARGET 2
#endif
#define CHECK(x) do { if (!(x)) { fprintf(stderr,"check failed at %d: %s\n",__LINE__,#x); return 1; } } while(0)
static cerune_process_bytes bytes(const void *p,size_t n) { return (cerune_process_bytes){p,n}; }
static int child(const char *mode) {
#ifdef _WIN32
    _setmode(_fileno(stdin),_O_BINARY);
    _setmode(_fileno(stdout),_O_BINARY);
    _setmode(_fileno(stderr),_O_BINARY);
#endif
    if (!strcmp(mode,"sleep")) {
#ifdef _WIN32
        Sleep(1000);
#else
        sleep(1);
#endif
        return 0;
    }
    unsigned char b[64]; size_t n=fread(b,1,sizeof b,stdin);
    if (fwrite(b,1,n,stdout)!=n) return 90;
    const unsigned char err[]={'e','r','r',0,13,10};
    if (fwrite(err,1,sizeof err,stderr)!=sizeof err) return 91;
    return 7;
}
int main(int argc,char **argv) {
    if (argc==3 && !strcmp(argv[1],"--child")) return child(argv[2]);
    CHECK(argc==2);
    FILE *f=fopen(argv[1],"rb"); CHECK(f);
    unsigned char config[65536]; size_t size=fread(config,1,sizeof config,f);
    CHECK(feof(f)); CHECK(fclose(f)==0);
    cerune_process_context *c=NULL;
    CHECK(cerune_process_v1_create(99,TARGET,bytes(config,size),&c)==CERUNE_PROCESS_VERSION && !c);
    CHECK(cerune_process_v1_create(1,3,bytes(config,size),&c)==CERUNE_PROCESS_TARGET && !c);
    CHECK(cerune_process_v1_create(1,TARGET,bytes("{",1),&c)==CERUNE_PROCESS_CONFIG && !c);
    CHECK(cerune_process_v1_create(1,TARGET,bytes(config,size),&c)==0 && c);
    memset(config,0,size); /* Context must own registration and limits. */
    char target[]="fixture", arg0[]="--child", mode[]="echo";
    unsigned char input[]={255,0,13,10};
    cerune_process_bytes args[]={bytes(arg0,7),bytes(mode,4)};
    cerune_process_request_v1 r={1,sizeof r,bytes(target,7),args,2,
        bytes(input,4),32,32,10000,123,9,30,41};
    uint64_t report[32], before[32], n=999;
    CHECK(cerune_process_v1_report(c,report,32)==CERUNE_PROCESS_STATE);
    r.version=2; CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_VERSION); r.version=1;
    r.size--; CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_SHAPE); r.size++;
    r.end=29; CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_RANGE); r.end=41;
    r.target=bytes("\xff",1); CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_UTF8);
    r.target=bytes(NULL,1); CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_SHAPE);
    r.target=bytes(target,7);
    CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_PENDING);
    CHECK(cerune_process_v1_prepare(c,&r)==CERUNE_PROCESS_STATE);
    CHECK(cerune_process_v1_report(c,report,31)==CERUNE_PROCESS_SHAPE);
    CHECK(cerune_process_v1_report(c,report,32)==0);
    CHECK(report[0]==1 && report[1]==32 && report[2]==123 && report[3]==9);
    CHECK(report[4]==30 && report[5]==41 && report[6]==0 && report[7]==4 && report[8]==0);
    CHECK(cerune_process_v1_execute(c,3)==CERUNE_PROCESS_RANGE);
    /* External storage is overwritten before execute. */
    memset(target,'x',7); memset(arg0,'x',7); memset(mode,'x',4); memset(input,0,4);
    CHECK(cerune_process_v1_execute(c,0)==CERUNE_PROCESS_READY);
    CHECK(cerune_process_v1_execute(c,0)==CERUNE_PROCESS_STATE);
    CHECK(cerune_process_v1_report(c,report,32)==0);
    CHECK(report[6]==1 && report[7]==4 && report[8]==4 && report[9]==32 && report[10]==32);
    CHECK(report[11]==10000 && report[12]==4 && report[13]==6 && report[14]==4 && report[15]==6);
    CHECK(report[16]==1 && report[17]==1 && report[18]==7 && report[19]==0);
    CHECK(report[20]==0 && report[21]==0 && report[22]==1 && report[25]==1);
    CHECK(report[24]==0 && report[27]==0);
    for (int i=28;i<32;i++) CHECK(report[i]==0);
    memcpy(before,report,sizeof report);
    unsigned char out[8]={0};
    CHECK(cerune_process_v1_copy(c,1,0,out,2,&n)==0 && n==2 && out[0]==255 && out[1]==0);
    CHECK(cerune_process_v1_copy(c,1,2,out,8,&n)==0 && n==2 && out[0]==13 && out[1]==10);
    CHECK(cerune_process_v1_copy(c,1,0,out,8,&n)==0 && n==4 && out[0]==255);
    CHECK(cerune_process_v1_copy(c,2,0,out,8,&n)==0 && n==6 && !memcmp(out,"err\0\r\n",6));
    CHECK(cerune_process_v1_copy(c,1,4,NULL,0,&n)==0 && n==0);
    CHECK(cerune_process_v1_copy(c,1,5,out,8,&n)==CERUNE_PROCESS_RANGE);
    CHECK(cerune_process_v1_report(c,report,32)==0 && !memcmp(report,before,sizeof report));
    CHECK(cerune_process_v1_release(c)==0);
    CHECK(cerune_process_v1_release(c)==CERUNE_PROCESS_STATE);
    memcpy(target,"fixture",7); memcpy(arg0,"--child",7); memcpy(mode,"echo",4);
    memcpy(input,"\xff\0\r\n",4);
    /* Discarding pending data is distinct from launch. */
    CHECK(cerune_process_v1_prepare(c,&r)==1); CHECK(cerune_process_v1_release(c)==0);
    for (int reservation=1;reservation<=2;reservation++) {
        CHECK(cerune_process_v1_prepare(c,&r)==1);
        CHECK(cerune_process_v1_execute(c,(uint64_t)reservation)==2);
        CHECK(cerune_process_v1_report(c,report,32)==0);
        CHECK(report[6]==0 && report[19]==(uint64_t)(reservation+5) && report[20]==2);
        CHECK(cerune_process_v1_release(c)==0);
    }
    r.timeout_ms=0;
    CHECK(cerune_process_v1_prepare(c,&r)==2);
    CHECK(cerune_process_v1_report(c,report,32)==0 && report[19]==1 && report[20]==1);
    CHECK(cerune_process_v1_release(c)==0); r.timeout_ms=10000;
    for (int missing=0;missing<2;missing++) {
        r.target=missing?bytes("missing",7):bytes("unknown",7);
        CHECK(cerune_process_v1_prepare(c,&r)==1);
        CHECK(cerune_process_v1_execute(c,0)==2);
        CHECK(cerune_process_v1_report(c,report,32)==0);
        CHECK(report[6]==0 && report[19]==(uint64_t)(missing?8:4) && report[20]==3);
        CHECK(cerune_process_v1_release(c)==0);
    }
    r.target=bytes(target,7); r.stdout_limit=2;
    CHECK(cerune_process_v1_prepare(c,&r)==1); CHECK(cerune_process_v1_execute(c,0)==2);
    CHECK(cerune_process_v1_report(c,report,32)==0 && report[19]==18 && report[14]==2 && report[12]==3);
    CHECK(cerune_process_v1_copy(c,1,0,out,8,&n)==0 && n==2 && out[0]==255 && out[1]==0);
    CHECK(cerune_process_v1_release(c)==0); r.stdout_limit=32;
    args[1]=bytes("sleep",5); r.timeout_ms=100;
    CHECK(cerune_process_v1_prepare(c,&r)==1); CHECK(cerune_process_v1_execute(c,0)==2);
    CHECK(cerune_process_v1_report(c,report,32)==0 && report[19]==19 && report[22]==1);
    const int unknown_progress=report[16]==0;
    CHECK(cerune_process_v1_release(c)==0);
    args[1]=bytes(mode,4); r.timeout_ms=10000;
    if (unknown_progress) {
        /* Releasing a result must not re-enable an uncertain connection. */
        CHECK(cerune_process_v1_prepare(c,&r)==2);
        CHECK(cerune_process_v1_report(c,report,32)==0 && report[19]==5 && report[6]==0);
    } else {
        CHECK(cerune_process_v1_prepare(c,&r)==1); CHECK(cerune_process_v1_execute(c,0)==2);
        CHECK(cerune_process_v1_report(c,report,32)==0 && report[19]==0 && report[18]==7);
    }
    CHECK(cerune_process_v1_destroy(&c)==0 && !c);
    CHECK(cerune_process_v1_destroy(&c)==CERUNE_PROCESS_SHAPE);
    CHECK(cerune_process_v1_create(1,TARGET,bytes(NULL,0),&c)==0);
    CHECK(cerune_process_v1_prepare(c,&r)==2);
    CHECK(cerune_process_v1_report(c,report,32)==0 && report[19]==3);
    CHECK(cerune_process_v1_destroy(&c)==0 && !c);
    puts("C ABI: ownership, states, origins, raw bytes, failures, cleanup: ok");
    return 0;
}
