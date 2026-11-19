BEGIN TRANSACTION;
CREATE TABLE [BOOKMARK-categories] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[order]	INTEGER,
					[category]	TEXT UNIQUE,
					[category_id]	INTEGER,
					[default] INTEGER
				);
INSERT INTO "BOOKMARK-categories" VALUES(1,1,'Reading',1,1);
CREATE TABLE [BOOKMARK-tabs] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[order]	INTEGER,
					[category_id]	INTEGER,
					[name]	TEXT,
					[url]	TEXT,
					[enabled]	INTEGER,
					[group_id]	INTEGER,
					[image]	TEXT,
					[background_color] TEXT,
					[text_color] TEXT
				);
INSERT INTO "BOOKMARK-tabs" VALUES(1,1,1,'News','https://news.example.com/',1,999,'plugins/images/tabs/news.png','#000000','#ffffff');
CREATE TABLE [categories] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[order]	INTEGER,
					[category]	TEXT UNIQUE,
					[category_id]	INTEGER,
					[image]	TEXT,
					[default] INTEGER
				);
INSERT INTO "categories" VALUES(1,1,'Unsorted',0,'fontawesome::question',1);
INSERT INTO "categories" VALUES(2,2,'Media',7,'fontawesome::film',0);
CREATE TABLE [chatroom] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[username]	TEXT,
					[gravatar]	TEXT,
					[uid]	TEXT,
					[date] DATETIME,
					[ip] TEXT,
					[message] TEXT
				);
INSERT INTO "chatroom" VALUES(1,'admin','','f1f17','2026-01-02 03:04:05','203.0.113.9','Hello');
CREATE TABLE [groups] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[group]	TEXT UNIQUE,
					[group_id]	INTEGER,
					[image]	TEXT,
					[default] INTEGER
				);
INSERT INTO "groups" VALUES(1,'Admin',0,'plugins/images/groups/admin.png',0);
INSERT INTO "groups" VALUES(2,'Co-Admin',1,'plugins/images/groups/coadmin.png',0);
INSERT INTO "groups" VALUES(3,'Super User',2,'plugins/images/groups/superuser.png',0);
INSERT INTO "groups" VALUES(4,'Power User',3,'plugins/images/groups/poweruser.png',0);
INSERT INTO "groups" VALUES(5,'User',4,'plugins/images/groups/user.png',1);
INSERT INTO "groups" VALUES(6,'Guest',999,'plugins/images/groups/guest.png',0);
INSERT INTO "groups" VALUES(7,'Editors',5,'plugins/images/groups/editors.png',0);
CREATE TABLE [invites] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[code]	TEXT UNIQUE,
					[date]	DATETIME,
					[email]	TEXT,
					[username]	TEXT,
					[dateused]	TIMESTAMP,
					[usedby]	TEXT,
					[ip]	TEXT,
					[valid]	TEXT,
					[type] TEXT,
					[invitedby] TEXT
				);
INSERT INTO "invites" VALUES(1,'MADEUP','2026-01-02 03:04:05','guest@example.com','guest',NULL,NULL,'203.0.113.9','Yes','Plex','admin');
CREATE TABLE [options] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[name]	TEXT UNIQUE,
					[value]	TEXT
				);
CREATE TABLE [tabs] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[order]	INTEGER,
					[category_id]	INTEGER,
					[name]	TEXT,
					[url]	TEXT,
					[url_local]	TEXT,
					[default]	INTEGER,
					[enabled]	INTEGER,
					[group_id]	INTEGER,
					[group_id_max] INTEGER DEFAULT '0',
					[add_to_admin]	INTEGER DEFAULT '0',
					[image]	TEXT,
					[type]	INTEGER,
					[splash]	INTEGER,
					[ping]		INTEGER,
					[ping_url]	TEXT,
					[timeout]	INTEGER,
					[timeout_ms]	INTEGER,
					[preload]	INTEGER
				);
INSERT INTO "tabs" VALUES(1,1,0,'Settings','api/v2/page/settings',NULL,0,1,1,0,0,'fontawesome::cog',0,NULL,NULL,NULL,NULL,NULL,NULL);
INSERT INTO "tabs" VALUES(2,2,0,'Homepage','api/v2/page/homepage',NULL,0,0,4,0,0,'fontawesome::home',0,NULL,NULL,NULL,NULL,NULL,NULL);
INSERT INTO "tabs" VALUES(3,3,7,'Plex','https://{domain}/plex','http://10.0.0.6:32400/web',1,1,4,0,0,'plugins/images/tabs/plex.png',1,NULL,1,'10.0.0.6:32400',NULL,NULL,1);
INSERT INTO "tabs" VALUES(4,4,0,'Docs','https://docs.example.com/',NULL,0,0,999,2,1,'fontawesome::book',2,NULL,NULL,NULL,NULL,NULL,NULL);
CREATE TABLE [tokens] (
					[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
					[token]	TEXT UNIQUE,
					[user_id]	INTEGER,
					[browser]	TEXT,
					[ip]	TEXT,
					[created] DATETIME,
					[expires] DATETIME
				);
INSERT INTO "tokens" VALUES(1,'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJPcmdhbml6ciIsImF1ZCI6Ik9yZ2FuaXpyIiwianRpIjoiNGYxZzIzYTEyYWEiLCJpYXQiOjE3OTUxMDkwMDUuNjc4MzM1LCJuYmYiOjE3OTUxMDkwMDUuNjc4MzM1LCJleHAiOjE3OTUxOTU0MDUuNjc4MzM1LCJuYW1lIjoiYWRtaW4iLCJncm91cCI6IkFkbWluIiwiZ3JvdXBJRCI6MCwiZW1haWwiOiJhZG1pbkBleGFtcGxlLmNvbSIsImltYWdlIjoiaHR0cHM6Ly93d3cuZ3JhdmF0YXIuY29tL2F2YXRhci9lNjRjN2Q4OWYyNmJkMTk3MmVmYTg1NGQxM2Q3ZGQ2MT9zPTEwMCZkPW1tIiwidXNlcklEIjoxfQ.IyF9uCtEy-hoP7JnHPCGMvK4EeIlfoHW18CG1o-KX4I',1,NULL,'172.18.0.1','2026-11-19 17:23:25','2026-11-20 17:23:25');
CREATE TABLE [users] (
						[id]	INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,
						[username]	TEXT UNIQUE,
						[password]	TEXT,
						[email]	TEXT,
						[plex_token]	TEXT,
						[group]	TEXT,
						[group_id]	INTEGER,
						[locked]	INTEGER,
						[image]	TEXT,
						[register_date]	DATETIME,
						[auth_service]	TEXT DEFAULT 'internal'
					);
INSERT INTO "users" VALUES(1,'admin','$2y$12$b3hMTpHv7o2Nf8Sif1wrTuR6vzYeIfjRE2/ZBhDEnxO3ZQ/sUTqSy','admin@example.com',NULL,'Admin',0,NULL,'https://www.gravatar.com/avatar/e64c7d89f26bd1972efa854d13d7dd61?s=100&d=mm','2026-11-19 17:23:25','internal');
INSERT INTO "users" VALUES(2,'robin','$2y$12$c5v0Eh7StvMU7PcTHq9hDO7YcOeRv9aIZgI4tozaFh8.L3HxzToQ.','robin@example.com',NULL,'User',4,NULL,'https://www.gravatar.com/avatar/d3c17ef52c397921a87c1de66f2311c2?s=100&d=mm','2026-11-19 17:23:25','google::JBSWY3DPEHPK3PXP');
INSERT INTO "users" VALUES(3,'pat','$2y$12$xsgaJD/kMZR3ul6v5Sn8B.rRWn.JVKGajGpwNrmpoX8SfoeUCT3zK','pat@example.com','made-up-token-of-a-plex-user','User',4,NULL,'https://www.gravatar.com/avatar/3ddcaf169f35457c40bc5960e69c4601?s=100&d=mm','2018-05-08T01:28:03Z','plex');
INSERT INTO "users" VALUES(4,'Sam','$2y$12$0EAg6Ork1B/oHqYyuVunPeB.Vwr5JRyx6Hqks7OvUw95eH/92NJuO','sam@example.com',NULL,'Super User',2,NULL,'https://www.gravatar.com/avatar/e179e95c00e7718ab4a23840f992ea63?s=100&d=mm','2026-11-19 17:23:26','internal');
DELETE FROM "sqlite_sequence";
INSERT INTO "sqlite_sequence" VALUES('users',4);
INSERT INTO "sqlite_sequence" VALUES('groups',7);
INSERT INTO "sqlite_sequence" VALUES('tabs',4);
INSERT INTO "sqlite_sequence" VALUES('categories',2);
INSERT INTO "sqlite_sequence" VALUES('tokens',1);
INSERT INTO "sqlite_sequence" VALUES('invites',1);
INSERT INTO "sqlite_sequence" VALUES('BOOKMARK-categories',1);
INSERT INTO "sqlite_sequence" VALUES('BOOKMARK-tabs',1);
INSERT INTO "sqlite_sequence" VALUES('chatroom',1);
COMMIT;
