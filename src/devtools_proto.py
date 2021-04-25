#!/usr/bin/env python3

import socket
from json import dumps, loads

HOST = '127.0.0.1'
PORT = 6081


def send(conn, obj):
    str = dumps(obj, separators=(',', ':'))
    print("<", str, "\n")
    conn.sendall(f"{len(str)}:{str}".encode())


def handle_incoming(conn, obj):
    tab = {"actor": "server1.conn1.tabDescriptor1", "browsingContextID": 1, "isZombieTab": False,
           "outerWindowID": 1, "selected": True, "title": "Main Window",
           "traits": {"watcher": True, "emitDescriptorDestroyed": True}, "url": "about:home"}

    if obj['type'] == "getRoot" and obj['to'] == "root":
        send(conn, {"preferenceActor": "server1.conn1.preferenceActor1",
                    "deviceActor": "server1.conn1.deviceActor1", "from": "root"})

    elif obj['type'] == "getDescription" and obj['to'] == 'server1.conn1.deviceActor1':
        send(conn, {
            "value": {"appid": "{ec8030f7-c20a-464f-9b0e-13a3a9e97384}", "apptype": "firefox", "vendor": "Mozilla",
                      "name": "Firefox", "version": "88.0", "appbuildid": "20210419141944",
                      "platformbuildid": "20210419141944", "geckobuildid": "20210419141944", "platformversion": "88.0",
                      "geckoversion": "88.0", "locale": "en-US", "endianness": "LE", "hostname": "pink", "os": "Linux",
                      "platform": "Linux", "hardware": "unknown", "deviceName": None, "arch": "x86_64",
                      "processor": "x86_64", "compiler": "gcc3", "profile": "test", "channel": "release",
                      "dpi": 187.93063354492188,
                      "useragent": "Mozilla/5.0 (X11; Linux x86_64; rv:88.0) Gecko/20100101 Firefox/88.0",
                      "width": 1920, "height": 1280, "physicalWidth": 1920, "physicalHeight": 1280,
                      "brandName": "Mozilla Firefox", "canDebugServiceWorkers": True},
            "from": "server1.conn1.deviceActor1"})
    elif obj['type'] == "getBoolPref" and obj['to'] == 'server1.conn1.preferenceActor1':
        send(conn, {"value": True, "from": "server1.conn1.preferenceActor1"})

    elif obj['type'] == 'listTabs' and obj['to'] == 'root':
        send(conn, {"tabs": [tab], "from": "root"})
    elif obj['type'] == 'listProcesses' and obj['to'] == 'root':
        send(conn, {"processes": [
            {"actor": "server1.conn1.processDescriptor1", "id": 0, "isParent": True,
             "traits": {"watcher": True}}
        ], "from": "root"})
    elif obj['type'].startswith("list") and obj['to'] == 'root':
        s = obj['type'].replace("list", "")
        if s == 'ServiceWorkerRegistrations':
            s = 'registrations'
        send(conn, {s[0].lower() + s[1:]: [], "from": "root"})
    elif obj['type'] == 'getProcess' and obj['to'] == 'root':
        send(conn, {"processDescriptor": {"actor": "server1.conn1.processDescriptor1", "id": 0, "isParent": True,
                                          "traits": {"watcher": True}}, "from": "root"})
    elif obj['type'] == 'getTab' and obj['to'] == 'root':
        send(conn, {"tab": tab, "from": "root"})

    elif obj['type'] == 'getFavicon' and obj['to'] == 'server1.conn1.tabDescriptor1':
        send(conn, {"favicon": None, "from": "server1.conn1.tabDescriptor1"})
    elif obj['type'] == 'getWatcher' and obj['to'] == 'server1.conn1.tabDescriptor1':
        send(conn, {
            "actor": "server1.conn1.watcher1", "traits": {
                "frame": True, "process": True, "worker": True,
                "resources": {
                    "console-message": True,
                    "css-change": True,
                    "css-message": True,
                    "document-event": True, "Cache": True,
                    "error-message": True,
                    "local-storage": True,
                    "session-storage": True,
                    "platform-message": True,
                    "network-event": True,
                    "network-event-stacktrace": True,
                    "stylesheet": True, "source": True,
                    "thread-state": True,
                    "server-sent-event": True,
                    "websocket": True
                },
                "set-breakpoints": True,
                "target-configuration": True,
                "thread-configuration": True,
                "set-xhr-breakpoints": True
            },
            "from": "server1.conn1.tabDescriptor1"
        })
    elif obj['type'] == 'getTarget' and obj['to'] == 'server1.conn1.tabDescriptor1':
        send(conn, {"frame": {"actor": "server1.conn1.child1/frameTarget1", "browsingContextID": 1,
                              "isTopLevelTarget": True,
                              "traits": {"isBrowsingContext": True, "reconfigureSupportsSimulationFeatures": True,
                                         "supportsTopLevelTargetFlag": True,
                                         "supportsFollowWindowGlobalLifeCycleFlag": True}, "title": "Main Window",
                              "url": "about:home", "outerWindowID": 1,
                              "consoleActor": "server1.conn1.child1/consoleActor1",
                              "inspectorActor": "server1.conn1.child1/inspectorActor1",
                              "styleSheetsActor": "server1.conn1.child1/styleSheetsActor1",
                              "storageActor": "server1.conn1.child1/storageActor1",
                              "memoryActor": "server1.conn1.child1/memoryActor1",
                              "framerateActor": "server1.conn1.child1/framerateActor1",
                              "reflowActor": "server1.conn1.child1/reflowActor1",
                              "cssPropertiesActor": "server1.conn1.child1/cssPropertiesActor1",
                              "performanceActor": "server1.conn1.child1/performanceActor1",
                              "animationsActor": "server1.conn1.child1/animationsActor1",
                              "responsiveActor": "server1.conn1.child1/responsiveActor1",
                              "webExtensionInspectedWindowActor": "server1.conn1.child1/webExtensionInspectedWindowActor1",
                              "accessibilityActor": "server1.conn1.child1/accessibilityActor1",
                              "changesActor": "server1.conn1.child1/changesActor1",
                              "webSocketActor": "server1.conn1.child1/webSocketActor1",
                              "eventSourceActor": "server1.conn1.child1/eventSourceActor1",
                              "manifestActor": "server1.conn1.child1/manifestActor1",
                              "networkContentActor": "server1.conn1.child1/networkContentActor1",
                              "screenshotContentActor": "server1.conn1.child1/screenshotContentActor1"},
                    "from": "server1.conn1.tabDescriptor1"})

    elif obj['type'] == 'attach' and obj['to'] == 'server1.conn1.child1/frameTarget1':
        send(conn, {"threadActor": "server1.conn1.child1/thread1", "cacheDisabled": False, "javascriptEnabled": True,
                    "traits": {"frames": True, "logInPage": True, "watchpoints": True, "navigation": True},
                    "from": "server1.conn1.child1/frameTarget1"})

    elif obj['type'] == 'startListeners' and obj['to'] == "server1.conn1.child1/consoleActor1":
        send(conn, {"startedListeners": [], "nativeConsoleAPI": True, "traits": {"blockedUrls": True},
                    "from": "server1.conn1.child1/consoleActor1"})

    elif obj['type'] == 'attach' and obj['to'] == 'server1.conn1.child1/thread1':
        send(conn, {"from": obj['to']})

    else:
        send(conn, {"from": obj['to']})


if __name__ == "__main__":
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind((HOST, PORT))
            s.listen()
            conn, addr = s.accept()
            with conn:
                send(conn, {"from": "root", "applicationType": "browser", "testConnectionPrefix": "server1.conn1.",
                            "traits": {"networkMonitor": False,
                                       "workerConsoleApiMessagesDispatchedToMainThread": False,
                                       "noPauseOnThreadActorAttach": False,
                                       "supportsThreadActorIsAttached": False}})

                while True:
                    len_str = ''
                    while (data := conn.recv(1)) != b':':
                        len_str += data.decode()

                    obj = loads(conn.recv(int(len_str)).decode())
                    print(">", dumps(obj, separators=(',', ':')))
                    handle_incoming(conn, obj)
    finally:
        s.close()
