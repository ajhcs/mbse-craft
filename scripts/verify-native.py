#!/usr/bin/env python3
"""Owned private X11 driver: actual pointer/keyboard input and captured pixels.
No ambient DISPLAY; no execution on import. Commands read from stdin as JSON.
"""
import argparse, ctypes as C, hashlib, json, os, pathlib, select, struct, subprocess, time, zlib
P=C.c_void_p; U=C.c_ulong
class Client(C.Structure):
    _fields_=[('type',C.c_int),('serial',U),('send_event',C.c_int),('display',P),('window',U),('message_type',U),('format',C.c_int),('data',C.c_long*5)]
class Event(C.Union):
    _fields_=[('client',Client),('pad',C.c_long*24)]
class Image(C.Structure):
    _fields_=[('width',C.c_int),('height',C.c_int),('xoffset',C.c_int),('format',C.c_int),('data',P),('byte_order',C.c_int),('bitmap_unit',C.c_int),('bitmap_bit_order',C.c_int),('bitmap_pad',C.c_int),('depth',C.c_int),('bytes_per_line',C.c_int),('bits_per_pixel',C.c_int),('red_mask',U),('green_mask',U),('blue_mask',U),('obdata',P),('functions',P*6)]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--exe',type=pathlib.Path,required=True);ap.add_argument('--run-dir',type=pathlib.Path,required=True);ap.add_argument('app_args',nargs=argparse.REMAINDER);a=ap.parse_args()
    assert str(a.run_dir).startswith('/workspace/') and a.exe.is_absolute()
    if not os.environ.get('MBSE_NATIVE_INNER'):
        a.run_dir.mkdir(parents=True,exist_ok=False);os.chmod(a.run_dir,0o700)
        (a.run_dir/'tmp').mkdir();(a.run_dir/'xkb').mkdir()
        runtime=pathlib.Path('/workspace/cache/builderbob/global-tools/releases/blender/5.2.2/xvfb')
        env=os.environ.copy();env['MBSE_NATIVE_INNER']='1'
        command=['/usr/bin/aa-exec','-p','builderbob-codex','--',str(runtime/'usr/bin/bwrap'),'--die-with-parent','--unshare-net','--ro-bind','/','/','--bind',str(a.run_dir),str(a.run_dir),'--bind',str(a.run_dir/'tmp'),'/tmp','--tmpfs','/usr/bin','--ro-bind','/usr/bin/dash','/usr/bin/sh','--ro-bind',str(runtime/'usr/bin/xkbcomp'),'/usr/bin/xkbcomp','--ro-bind',str(pathlib.Path(__import__('sys').executable).resolve()),str(pathlib.Path(__import__('sys').executable).resolve()),'--dev-bind','/dev','/dev','--tmpfs','/var','--bind',str(a.run_dir/'xkb'),'/var/lib/xkb','--',str(pathlib.Path(__import__('sys').executable).resolve()),str(pathlib.Path(__file__).resolve()),*__import__('sys').argv[1:]]
        raise SystemExit(subprocess.call(command,env=env))
    auth=a.run_dir/'Xauthority'; fields=(b'',b'',b'MIT-MAGIC-COOKIE-1',os.urandom(16));auth.write_bytes(struct.pack('>H',65535)+b''.join(struct.pack('>H',len(v))+v for v in fields));os.chmod(auth,0o600)
    env=os.environ.copy();env['XAUTHORITY']=str(auth);env['LIBGL_ALWAYS_SOFTWARE']='1';env['WINIT_UNIX_BACKEND']='x11';env['XDG_RUNTIME_DIR']=str(a.run_dir)
    runtime=pathlib.Path('/workspace/cache/builderbob/global-tools/releases/blender/5.2.2/xvfb')
    env['PATH']=str(runtime/'usr/bin')+':'+env['PATH']; env['LD_LIBRARY_PATH']=str(runtime/'usr/lib/x86_64-linux-gnu')+':/workspace/cache/builderbob/waypoint-desktop-x11/runtime/usr/lib/x86_64-linux-gnu:'+env.get('LD_LIBRARY_PATH','')
    r,w=os.pipe();server=app=None;log=(a.run_dir/'events.jsonl').open('x')
    def event(v): log.write(json.dumps(v)+'\n');log.flush();print(json.dumps(v),flush=True)
    try:
        server=subprocess.Popen([str(runtime/'usr/bin/Xvfb'),'-displayfd',str(w),'-screen','0','1600x1200x24','-nolisten','tcp','-auth',str(auth),'-xkbdir','/usr/share/X11/xkb'],pass_fds=(w,),env=env,stdout=(a.run_dir/'xvfb.log').open('x'),stderr=subprocess.STDOUT);os.close(w)
        assert select.select([r],[],[],15)[0],'Xvfb timeout';number=os.read(r,64).decode().strip();os.close(r);assert number.isdigit();env['DISPLAY']=':'+number;os.environ['XAUTHORITY']=str(auth)
        x=C.CDLL('libX11.so.6');xt=C.CDLL('libXtst.so.6')
        signatures={'XSendEvent':(C.c_int,[P,U,C.c_int,C.c_long,C.POINTER(Event)]),'XOpenDisplay':(P,[C.c_char_p]),'XDefaultRootWindow':(U,[P]),'XQueryTree':(C.c_int,[P,U,C.POINTER(U),C.POINTER(U),C.POINTER(C.POINTER(U)),C.POINTER(C.c_uint)]),'XInternAtom':(U,[P,C.c_char_p,C.c_int]),'XGetWindowProperty':(C.c_int,[P,U,U,C.c_long,C.c_long,C.c_int,U,C.POINTER(U),C.POINTER(C.c_int),C.POINTER(U),C.POINTER(U),C.POINTER(P)]),'XFree':(C.c_int,[P]),'XSetInputFocus':(C.c_int,[P,U,C.c_int,U]),'XSync':(C.c_int,[P,C.c_int]),'XStringToKeysym':(U,[C.c_char_p]),'XKeysymToKeycode':(C.c_ubyte,[P,U]),'XGetImage':(C.POINTER(Image),[P,U,C.c_int,C.c_int,C.c_uint,C.c_uint,U,C.c_int]),'XDestroyImage':(C.c_int,[C.POINTER(Image)])}
        for name,(rest,args) in signatures.items():f=getattr(x,name);f.restype=rest;f.argtypes=args
        for name,args in {'XTestFakeMotionEvent':[P,C.c_int,C.c_int,C.c_int,U],'XTestFakeButtonEvent':[P,C.c_uint,C.c_int,U],'XTestFakeKeyEvent':[P,C.c_uint,C.c_int,U]}.items():f=getattr(xt,name);f.restype=C.c_int;f.argtypes=args
        d=x.XOpenDisplay(env['DISPLAY'].encode());assert d;root=x.XDefaultRootWindow(d)
        args=a.app_args[1:] if a.app_args[:1]==['--'] else a.app_args
        app=subprocess.Popen([str(a.exe),'--state-dir',str(a.run_dir/'state'),*args],env=env,stdout=(a.run_dir/'app.log').open('x'),stderr=subprocess.STDOUT)
        def children(win):
            rr=U();parent=U();items=C.POINTER(U)();n=C.c_uint();x.XQueryTree(d,win,C.byref(rr),C.byref(parent),C.byref(items),C.byref(n));result=[items[i] for i in range(n.value)]
            if items:x.XFree(items)
            return result
        def pid(win):
            typ=U();fmt=C.c_int();n=U();left=U();data=P();x.XGetWindowProperty(d,win,x.XInternAtom(d,b'_NET_WM_PID',0),0,1,0,0,C.byref(typ),C.byref(fmt),C.byref(n),C.byref(left),C.byref(data));v=C.cast(data,C.POINTER(U))[0] if n.value else None
            if data:x.XFree(data)
            return v
        win=None
        for _ in range(150):
            if app.poll() is not None:raise RuntimeError('App exited: '+(a.run_dir/'app.log').read_text())
            for candidate in children(root):
                if pid(candidate)==app.pid:win=candidate;break
            if win:break
            time.sleep(.1)
        assert win,'owned window timeout';time.sleep(2);x.XSetInputFocus(d,win,1,0);x.XSync(d,0);time.sleep(1)
        event({'ready':True,'app_pid':app.pid,'server_pid':server.pid,'window':win,'binary_sha256':hashlib.sha256(a.exe.read_bytes()).hexdigest()})
        def key(name,down):
            code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code, name;xt.XTestFakeKeyEvent(d,code,down,0)
        for line in __import__('sys').stdin:
            cmd=json.loads(line);op=cmd['op']
            if op=='stop':break
            assert app.poll() is None and pid(win)==app.pid
            if op=='click':
                if 'widget' in cmd:
                    audit=json.loads((a.run_dir/'state/ui-geometry.json').read_text());rect=audit['widgets'][cmd['widget']];scale=audit['pixels_per_point'];assert rect[2]>rect[0] and rect[3]>rect[1], 'widget clipped';cmd['x']=int((rect[0]+rect[2])*scale/2);cmd['y']=int((rect[1]+rect[3])*scale/2)
                xt.XTestFakeMotionEvent(d,0,cmd['x'],cmd['y'],0);xt.XTestFakeButtonEvent(d,1,1,0);xt.XTestFakeButtonEvent(d,1,0,0)
            elif op=='close':
                e=Event();e.client.type=33;e.client.send_event=1;e.client.display=d;e.client.window=win;e.client.message_type=x.XInternAtom(d,b'WM_PROTOCOLS',0);e.client.format=32;e.client.data[0]=x.XInternAtom(d,b'WM_DELETE_WINDOW',0);e.client.data[1]=0;assert x.XSendEvent(d,win,0,0,C.byref(e))
            elif op=='scroll':
                xt.XTestFakeMotionEvent(d,0,cmd['x'],cmd['y'],0)
                for _ in range(abs(cmd['steps'])):
                    button=5 if cmd['steps']>0 else 4;xt.XTestFakeButtonEvent(d,button,1,0);xt.XTestFakeButtonEvent(d,button,0,0)
            elif op=='key':
                for mod in cmd.get('mods',[]):key(mod,1)
                key(cmd['key'],1);key(cmd['key'],0)
                for mod in reversed(cmd.get('mods',[])):key(mod,0)
            elif op=='text':
                punctuation={' ':'space','/':'slash','.':'period','-':'minus',':':'colon','_':'underscore','+':'plus','[':'bracketleft',']':'bracketright','*':'asterisk','=':'equal'}
                for char in cmd['text']:
                    shift=char.isupper() or char in ':_+*';name=punctuation.get(char,char.lower());
                    if shift:key('Shift_L',1)
                    key(name,1);key(name,0)
                    if shift:key('Shift_L',0)
            elif op=='capture':
                width=cmd.get('width',1440);height=cmd.get('height',1000);im=x.XGetImage(d,win,0,0,width,height,U(-1).value,2);assert im
                try:
                    v=im.contents;assert (v.bits_per_pixel,v.byte_order,v.red_mask,v.green_mask,v.blue_mask)==(32,0,0xff0000,0xff00,0xff)
                    raw=C.string_at(v.data,v.bytes_per_line*height);rows=bytearray()
                    for y in range(height):
                        row=raw[y*v.bytes_per_line:y*v.bytes_per_line+width*4];rgb=bytearray(width*3);rgb[0::3],rgb[1::3],rgb[2::3]=row[2::4],row[1::4],row[0::4];rows.extend(b'\0'+rgb)
                    def chunk(k,b):return struct.pack('>I',len(b))+k+b+struct.pack('>I',zlib.crc32(k+b))
                    data=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(rows,3))+chunk(b'IEND',b'');path=a.run_dir/(cmd['name']+'.png');path.write_bytes(data);cmd['sha256']=hashlib.sha256(data).hexdigest()
                finally:x.XDestroyImage(im)
            elif op=='stop':break
            else:raise ValueError(op)
            x.XSync(d,0);time.sleep(cmd.get('pause',.35));cmd['app_returncode']=app.poll();event(cmd)
    finally:
        for child in (app,server):
            if child and child.poll() is None:child.terminate();child.wait(timeout=8)
        auth.unlink(missing_ok=True);log.close()
if __name__=='__main__':main()
