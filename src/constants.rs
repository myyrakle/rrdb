// 기본 데이터베이스 이름
pub const DEFAULT_DATABASE_NAME: &str = "rrdb";

// 기본 설정파일 이름.
pub const DEFAULT_CONFIG_FILENAME: &str = "rrdb.config";

// 기본 Data 디렉터리 이름
pub const DEFAULT_DATA_DIRNAME: &str = "data";

// 운영체제별 기본 저장 경로를 반환합니다.
#[cfg(target_os = "linux")]
pub const DEFAULT_CONFIG_BASEPATH: &str = "/var/lib/rrdb";

#[cfg(target_os = "windows")]
pub const DEFAULT_CONFIG_BASEPATH: &str = r"C:\Program Files\rrdb";

#[cfg(target_os = "macos")]
pub const DEFAULT_CONFIG_BASEPATH: &str = "/var/lib/rrdb";

pub const LAUNCHD_PLIST_PATH: &str = "/Library/LaunchDaemons/io.github.myyrakle.rrdb.plist";

#[cfg(target_os = "linux")]
pub const SYSTEMD_DAEMON_SCRIPT: &str = r#"[Unit]
Description=RRDB

[Service]
Type=simple
Restart=on-failure
ExecStart=/usr/bin/rrdb run
RemainAfterExit=on
User=root
StandardOutput=file:/var/log/rrdb.stdout.log
StandardError=file:/var/log/rrdb.stderr.log

[Install]
WantedBy=multi-user.target"#;

#[cfg(target_os = "macos")]
pub const LAUNCHD_DAEMON_SCRIPT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
        <key>Label</key>
        <string>myyrakle.github.io.rrdb</string>
        <key>UserName</key>
        <string>root</string>
        <key>Program</key>
        <string>/usr/local/bin/rrdb</string>
        <key>ProgramArguments</key>
        <array>
            <string>run</string>
        </array>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardOutPath</key>
        <string>/var/log/rrdb.stdout.log</string>
        <key>StandardErrorPath</key>
        <string>/var/log/rrdb.stderr.log</string>
</dict>
</plist>"#;
