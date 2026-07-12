// File: src/App.tsx

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import "./App.css";

interface Rule {
  id?: number;
  pattern: string;
  match_type: string;
  category: string;
}

interface ReportItem {
  process_name: string;
  window_title: string;
  category: string;
  total_duration_sec: number;
  project_tag?: string;
}

interface TimelineItem {
  hour: number;
  category: string;
  duration: number;
}

// ==========================================
// 1. 동기식 웹뷰 완전 투명화 및 하단 뷰포트 배너 컴포넌트
// ==========================================
function WarningApp() {
  useEffect(() => {
    // 윈도우 OS 웹뷰가 지닌 하얀색 배경 도화지 자체를 완전히 투명하게 날려버립니다.
    document.body.style.backgroundColor = "transparent";
    document.body.style.background = "transparent";
    document.documentElement.style.backgroundColor = "transparent";
    document.documentElement.style.background = "transparent";

    return () => {
      document.body.style.backgroundColor = "";
      document.body.style.background = "";
      document.documentElement.style.backgroundColor = "";
      document.documentElement.style.background = "";
    };
  }, []);

  return (
    <div className="fullscreen-warning-body">
      {/* 🌟 크롬 상단 탭/주소창 영역(약 85px) 밑으로만 정확히 테두리 오버레이를 얹습니다! */}
      <div className="tab-below-warning-overlay">
        <div className="warning-capsule">
          <span className="warning-icon">⚠️</span>
          <span>집중 모드 작동 중! 이 탭을 종료하거나 업무 창으로 복귀하면 이 알림은 자동으로 사라집니다.</span>
        </div>
      </div>
    </div>
  );
}

// ==========================================
// 2. 메인 대시보드 애플리케이션 컴포넌트
// ==========================================
function DashboardApp() {
  const [activeTab, setActiveTab] = useState<"dashboard" | "rules" | "settings">("dashboard");
  const [filterDays, setFilterDays] = useState<number>(1);
  const [report, setReport] = useState<ReportItem[]>([]);
  const [rules, setRules] = useState<Rule[]>([]);
  const [isAutostart, setIsAutostart] = useState<boolean>(false);
  const [isDarkMode, setIsDarkMode] = useState<boolean>(false);
  const [isGroupedByDomain, setIsGroupedByDomain] = useState<boolean>(true);

  // 프로젝트 태그 데이터 관리 (로컬스토리지 저장)
  const [projects, setProjects] = useState<string[]>(() => {
    const saved = localStorage.getItem("projects");
    return saved ? JSON.parse(saved) : ["회사 일과", "개인 프로젝트"];
  });
  const [selectedProject, setSelectedProject] = useState<string>("ALL");
  const [newProjectName, setNewProjectName] = useState("");

  // 24시간 타임라인 데이터 수집
  const [timeline, setTimeline] = useState<TimelineItem[]>([]);

  // Gist 클라우드 동기화 설정 (로컬 보관)
  const [githubPat, setGithubPat] = useState(() => localStorage.getItem("github_pat") || "");
  const [gistId, setGistId] = useState(() => localStorage.getItem("gist_id") || "");
  const [syncStatus, setSyncStatus] = useState("");

  // 뽀모도로 타이머 세션 관리
  const [focusTimeLeft, setFocusTimeLeft] = useState<number>(0); 
  const [isFocusActive, setIsFocusActive] = useState<boolean>(false);
  const [focusDuration, setFocusDuration] = useState<number>(15 * 60);

  // 규칙 생성 입력용 상태 값
  const [newPattern, setNewPattern] = useState("");
  const [newMatchType, setNewMatchType] = useState("CONTAINS");
  const [newCategory, setNewCategory] = useState("PRODUCTIVE");

  // 지능형 모달 제어 상태
  const [deleteTarget, setDeleteTarget] = useState<ReportItem | null>(null);

  // 테마 초기화 설정
  useEffect(() => {
    const savedTheme = localStorage.getItem("theme");
    if (savedTheme === "dark") {
      setIsDarkMode(true);
      document.body.classList.add("dark-mode");
    }
  }, []);

  // 테마 토글
  const toggleTheme = () => {
    if (isDarkMode) {
      setIsDarkMode(false);
      document.body.classList.remove("dark-mode");
      localStorage.setItem("theme", "light");
    } else {
      setIsDarkMode(true);
      document.body.classList.add("dark-mode");
      localStorage.setItem("theme", "dark");
    }
  };

  // 데이터 로드
  const loadReport = async () => {
    try {
      const data: ReportItem[] = await invoke("get_report", { days: filterDays });
      setReport(data);
    } catch (err) {
      console.error(err);
    }
  };

  const loadRules = async () => {
    try {
      const data: Rule[] = await invoke("get_rules");
      setRules(data);
    } catch (err) {
      console.error(err);
    }
  };

  const loadTimeline = async () => {
    try {
      const data: TimelineItem[] = await invoke("get_timeline");
      setTimeline(data);
    } catch (err) {
      console.error(err);
    }
  };

  const checkAutostart = async () => {
    try {
      const enabled = await isEnabled();
      setIsAutostart(enabled);
    } catch (err) {
      console.error(err);
    }
  };

  const handleProjectSelect = async (tag: string) => {
    setSelectedProject(tag);
    await invoke("set_current_project", { tag: tag === "ALL" ? null : tag });
    loadReport();
  };

  const handleAddProject = () => {
    if (!newProjectName.trim()) return;
    const updated = [...projects, newProjectName.trim()];
    setProjects(updated);
    localStorage.setItem("projects", JSON.stringify(updated));
    setNewProjectName("");
  };

  const startFocusSession = () => {
    setFocusTimeLeft(focusDuration);
    setIsFocusActive(true);
  };

  const stopFocusSession = async () => {
    setIsFocusActive(false);
    setFocusTimeLeft(0);
    await invoke("toggle_focus_warning", { show: false });
  };

  useEffect(() => {
    loadReport();
    loadRules();
    loadTimeline();
    checkAutostart();
  }, [filterDays, selectedProject]);

  useEffect(() => {
    const timer = setInterval(() => {
      if (activeTab === "dashboard") {
        loadReport();
        loadTimeline();
      }
    }, 3000);
    return () => clearInterval(timer);
  }, [activeTab, filterDays]);

  // 뽀모도로 세션 실시간 1초 판독 연동 루프 및 포기 이벤트 바인딩
  useEffect(() => {
    let interval: any; 
    let unlistenFn: (() => void) | null = null;

    const setupEvent = async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlistenFn = await listen("cancel-focus-session", () => {
          setIsFocusActive(false);
          setFocusTimeLeft(0);
          invoke("toggle_focus_warning", { show: false });
        });
      } catch (e) {
        console.error("이벤트 바인딩 실패", e);
      }
    };
    setupEvent();

    if (isFocusActive && focusTimeLeft > 0) {
      interval = setInterval(async () => {
        setFocusTimeLeft((prev) => prev - 1);

        try {
          const activeCategory: string = await invoke("get_current_active_category");
          
          if (activeCategory === "UNPRODUCTIVE") {
            await invoke("toggle_focus_warning", { show: true });
          } else {
            await invoke("toggle_focus_warning", { show: false });
          }
        } catch (err) {
          console.error("실시간 오버레이 체커 구동 실패:", err);
        }
      }, 1000);
    } else if (focusTimeLeft === 0 && isFocusActive) {
      setIsFocusActive(false);
      invoke("toggle_focus_warning", { show: false });
    }

    return () => {
      clearInterval(interval);
      if (unlistenFn) unlistenFn();
    };
  }, [isFocusActive, focusTimeLeft]);

  // Gist 클라우드 업로드 백업 연동
  const handleCloudBackup = async () => {
    if (!githubPat.trim()) {
      setSyncStatus("에러: GitHub 개인 토큰(PAT)을 설정해 주세요.");
      return;
    }
    setSyncStatus("백업 업로드 중...");
    try {
      const bytes: number[] = await invoke("get_db_bytes");
      
      let binary = "";
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      const base64Content = window.btoa(binary);

      const payload = {
        description: "Focus Tracker Cloud Backup Data",
        public: false,
        files: {
          "focus_tracker_backup.db.b64": {
            content: base64Content,
          },
        },
      };

      const url = gistId ? `https://api.github.com/gists/${gistId}` : "https://api.github.com/gists";
      const method = gistId ? "PATCH" : "POST";

      const res = await fetch(url, {
        method,
        headers: {
          Authorization: `token ${githubPat}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      });

      if (res.ok) {
        const data = await res.json();
        localStorage.setItem("gist_id", data.id);
        setGistId(data.id);
        setSyncStatus("클라우드 백업 성공!");
      } else {
        setSyncStatus(`에러: 백업 응답 실패 (${res.status})`);
      }
    } catch (err: any) {
      setSyncStatus(`백업 에러: ${err}`);
    }
  };

  // Gist 클라우드 다운로드 동기화 복구 연동
  const handleCloudRestore = async () => {
    if (!githubPat.trim() || !gistId.trim()) {
      setSyncStatus("에러: 토큰 및 Gist ID를 먼저 설정해 주세요.");
      return;
    }
    setSyncStatus("동기화 내려받는 중...");
    try {
      const res = await fetch(`https://api.github.com/gists/${gistId}`, {
        headers: { Authorization: `token ${githubPat}` },
      });

      if (res.ok) {
        const data = await res.json();
        const base64Content = data.files["focus_tracker_backup.db.b64"]?.content;
        
        if (!base64Content) {
          setSyncStatus("에러: 백업 파일 형식이 손상되었습니다.");
          return;
        }

        const binaryString = window.atob(base64Content.replace(/\s/g, ""));
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }

        const arrayData = Array.from(bytes);
        await invoke("restore_db_bytes", { bytes: arrayData });

        setSyncStatus("클라우드 동기화 완료! 통계가 복원되었습니다.");
        loadReport();
        loadTimeline();
      } else {
        setSyncStatus("동기화 에러: 다운로드 응답 실패");
      }
    } catch (err: any) {
      setSyncStatus(`동기화 에러: ${err}`);
    }
  };

  const handleAddRule = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPattern.trim()) return;
    try {
      await invoke("add_rule", {
        pattern: newPattern,
        matchType: newMatchType,
        category: newCategory,
      });
      setNewPattern("");
      loadRules();
    } catch (err) {
      console.error(err);
    }
  };

  const handleDeleteRule = async (id: number) => {
    try {
      await invoke("delete_rule", { id });
      loadRules();
    } catch (err) {
      console.error(err);
    }
  };

  const handleInlineCategoryChange = async (title: string, oldCategory: string, newCategory: string) => {
    if (oldCategory === newCategory) return;
    const targetItem = report.find((i) => i.window_title === title);
    if (newCategory === "IGNORE") {
      if (targetItem) setDeleteTarget(targetItem);
      return;
    }
    try {
      let pattern = title.split(" - ")[0].split(" | ")[0].trim();
      if (!pattern && targetItem) {
        pattern = targetItem.process_name;
      }
      await invoke("update_category_and_rule_cmd", {
        pattern,
        matchType: "CONTAINS",
        category: newCategory,
      });
      loadReport();
      loadRules();
    } catch (err) {
      console.error(err);
    }
  };

  const executeDeleteAndIgnore = async (shouldIgnore: boolean) => {
    if (!deleteTarget) return;
    try {
      let pattern = deleteTarget.window_title.split(" - ")[0].split(" | ")[0].trim();
      if (!pattern) {
        pattern = deleteTarget.process_name;
      }
      await invoke("delete_and_ignore_item_cmd", {
        pattern,
        matchType: "CONTAINS",
        shouldIgnore,
      });
      setDeleteTarget(null);
      loadReport();
      loadRules();
    } catch (err) {
      console.error(err);
    }
  };

  const handleExportCSV = async () => {
    try {
      await invoke("export_report_csv", { days: filterDays });
    } catch (err) {
      console.error(err);
    }
  };

  const handleToggleAutostart = async () => {
    try {
      if (isAutostart) {
        await disable();
        setIsAutostart(false);
      } else {
        await enable();
        setIsAutostart(true);
      }
    } catch (err) {
      console.error(err);
    }
  };

  const parseDomain = (windowTitle: string, processName: string): string => {
    const title = windowTitle.toLowerCase();
    const isBrowser = processName.toLowerCase().includes("chrome") ||
                      processName.toLowerCase().includes("edge") ||
                      processName.toLowerCase().includes("msedge") ||
                      processName.toLowerCase().includes("brave") ||
                      processName.toLowerCase().includes("firefox") ||
                      processName.toLowerCase().includes("opera") ||
                      processName.toLowerCase().includes("whale");

    if (!isBrowser) return processName;

    if (title.includes("github")) return "github.com";
    if (title.includes("ai studio") || title.includes("aistudio")) return "aistudio.google.com";
    if (title.includes("codex") || title.includes("chatgpt")) return "chatgpt.com";
    if (title.includes("youtube")) return "youtube.com";
    if (title.includes("instagram")) return "instagram.com";
    if (title.includes("sheets")) return "sheets.google.com";
    if (title.includes("docs")) return "docs.google.com";
    if (title.includes("gmail") || title.includes("mail.google")) return "mail.google.com";
    if (title.includes("naver")) return "naver.com";
    if (title.includes("google")) return "google.com";
    if (title.includes("stackoverflow") || title.includes("stack overflow")) return "stackoverflow.com";
    if (title.includes("tistory")) return "tistory.com";
    if (title.includes("velog")) return "velog.io";

    const segments = windowTitle.split(/[|\-·]/);
    if (segments.length > 1) {
      const mainPart = segments[0].trim();
      if (mainPart) return mainPart;
    }
    return windowTitle.trim() === "" ? "인터넷 브라우저" : windowTitle.trim();
  };

  const getDisplayReport = (): ReportItem[] => {
    let filtered = report;
    if (selectedProject !== "ALL") {
      filtered = report.filter((item) => item.project_tag === selectedProject);
    }

    if (!isGroupedByDomain) {
      return filtered;
    }

    const map = new Map<string, ReportItem>();
    for (const item of filtered) {
      const domain = parseDomain(item.window_title, item.process_name);
      const key = `${item.process_name}::${domain}::${item.category}`;

      if (map.has(key)) {
        const existing = map.get(key)!;
        existing.total_duration_sec += item.total_duration_sec;
      } else {
        map.set(key, {
          process_name: item.process_name,
          window_title: domain,
          category: item.category,
          total_duration_sec: item.total_duration_sec,
          project_tag: item.project_tag,
        });
      }
    }
    return Array.from(map.values()).sort((a, b) => b.total_duration_sec - a.total_duration_sec);
  };

  const formatDuration = (totalSeconds: number) => {
    if (totalSeconds < 60) return `${totalSeconds}초`;
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    if (minutes < 60) return `${minutes}분 ${seconds}초`;
    const hours = Math.floor(minutes / 60);
    const remainMinutes = minutes % 60;
    return `${hours}시간 ${remainMinutes}분`;
  };

  const targetReport = selectedProject === "ALL" ? report : report.filter(i => i.project_tag === selectedProject);

  const totalProductive = targetReport
    .filter((item) => item.category === "PRODUCTIVE")
    .reduce((sum, item) => sum + item.total_duration_sec, 0);

  const totalUnproductive = targetReport
    .filter((item) => item.category === "UNPRODUCTIVE")
    .reduce((sum, item) => sum + item.total_duration_sec, 0);

  const totalNeutral = targetReport
    .filter((item) => item.category === "NEUTRAL")
    .reduce((sum, item) => sum + item.total_duration_sec, 0);

  const totalTrackedTime = totalProductive + totalUnproductive + totalNeutral;
  const productivityScore =
    totalTrackedTime > 0
      ? Math.round((totalProductive / (totalProductive + totalUnproductive || 1)) * 100)
      : 0;

  const prodRatio = totalTrackedTime > 0 ? (totalProductive / totalTrackedTime) * 100 : 0;
  const unprodRatio = totalTrackedTime > 0 ? (totalUnproductive / totalTrackedTime) * 100 : 0;
  const neutRatio = totalTrackedTime > 0 ? (totalNeutral / totalTrackedTime) * 100 : 0;

  const radius = 40;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (productivityScore / 100) * circumference;

  const displayReport = getDisplayReport();
  const topFiveDomains = displayReport.slice(0, 5);
  const maxTimeValue = topFiveDomains.length > 0 ? topFiveDomains[0].total_duration_sec : 1;

  return (
    <div className="app-container">
      <header>
        <h1>⏰ Focus Tracker</h1>
        <div className="tabs-row">
          <div className="tabs">
            <button className={`tab-btn ${activeTab === "dashboard" ? "active" : ""}`} onClick={() => setActiveTab("dashboard")}>대시보드</button>
            <button className={`tab-btn ${activeTab === "rules" ? "active" : ""}`} onClick={() => setActiveTab("rules")}>규칙 설정</button>
            <button className={`tab-btn ${activeTab === "settings" ? "active" : ""}`} onClick={() => setActiveTab("settings")}>환경 설정</button>
          </div>
          <button className="theme-toggle-btn" onClick={toggleTheme} title="테마 변경">
            {isDarkMode ? "☀️" : "🌙"}
          </button>
        </div>
      </header>

      <main>
        {activeTab === "dashboard" && (
          <div>
            {/* 뽀모도로 집중 세션 상단 통제 바 */}
            <div className="pomodoro-status-bar">
              <div>
                <strong>집중 차단 모드 타이머</strong>
                <p style={{ fontSize: "0.85rem", color: "var(--neutral)", margin: "0.2rem 0 0 0" }}>
                  지정한 시간 동안 비생산적인 딴짓 행동을 완벽하게 차단하고 통제합니다.
                </p>
              </div>
              <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
                {!isFocusActive ? (
                  <>
                    <select
                      className="select-filter"
                      style={{ padding: "0.4rem 0.75rem" }}
                      value={focusDuration}
                      onChange={(e) => setFocusDuration(Number(e.target.value))}
                    >
                      <option value={15 * 60}>15분 집중</option>
                      <option value={30 * 60}>30분 집중</option>
                      <option value={50 * 60}>50분 집중</option>
                    </select>
                    <button className="btn-primary" onClick={startFocusSession}>⏰ 집중 모드 가동</button>
                  </>
                ) : (
                  <>
                    <span className="pomodoro-time">
                      {Math.floor(focusTimeLeft / 60)}분 {focusTimeLeft % 60}초 남음
                    </span>
                    <button className="btn-danger" style={{ padding: "0.5rem 1rem", borderRadius: "6px" }} onClick={stopFocusSession}>포기/취소</button>
                  </>
                )}
              </div>
            </div>

            {/* 통계 요약 그리드 */}
            <div className="stats-grid">
              <div className="stat-card productive">
                <div className="stat-label">생산적 업무</div>
                <div className="stat-value">{formatDuration(totalProductive)}</div>
              </div>
              <div className="stat-card unproductive">
                <div className="stat-label">비생산적 시간</div>
                <div className="stat-value">{formatDuration(totalUnproductive)}</div>
              </div>
              <div className="stat-card">
                <div className="stat-label">총 측정 누적</div>
                <div className="stat-value">{formatDuration(totalTrackedTime)}</div>
              </div>
            </div>

            {/* 필터 조작 줄 및 프로젝트 태깅 제어기 */}
            <div className="control-row">
              <div style={{ display: "flex", gap: "1rem" }}>
                <select className="select-filter" value={filterDays} onChange={(e) => setFilterDays(Number(e.target.value))}>
                  <option value={1}>오늘 (24시간)</option>
                  <option value={7}>최근 7일</option>
                  <option value={30}>최근 30일 (1달)</option>
                </select>

                <div className="toggle-btn-group">
                  <button className={`toggle-item ${isGroupedByDomain ? "active" : ""}`} onClick={() => setIsGroupedByDomain(true)}>도메인 묶어보기</button>
                  <button className={`toggle-item ${!isGroupedByDomain ? "active" : ""}`} onClick={() => setIsGroupedByDomain(false)}>상세 내역 전체보기</button>
                </div>

                {/* 현재 활성 프로젝트 태그 관리 드롭다운 */}
                <select
                  className="select-filter"
                  style={{ borderColor: "var(--productive)" }}
                  value={selectedProject}
                  onChange={(e) => handleProjectSelect(e.target.value)}
                >
                  <option value="ALL">📂 전체 프로젝트 보기</option>
                  {projects.map((proj, idx) => (
                    <option key={idx} value={proj}>📂 {proj}</option>
                  ))}
                </select>
              </div>

              <button className="btn-primary" onClick={handleExportCSV}>📄 CSV 업무보고서 저장</button>
            </div>

            {/* 대시보드 2분할 뷰 개편 */}
            <div className="dashboard-layout">
              <div className="dashboard-sidebar">
                {/* 도넛 진척 차트 */}
                <div className="card doughnut-card">
                  <div className="stat-label">하루 생산성 도넛</div>
                  <div className="doughnut-svg-container">
                    <svg width="100%" height="100%" viewBox="0 0 100 100">
                      <circle cx="50" cy="50" r="40" fill="transparent" stroke="var(--border)" strokeWidth="10" />
                      <circle cx="50" cy="50" r="40" fill="transparent" stroke="var(--productive)" strokeWidth="10"
                        strokeDasharray={circumference} strokeDashoffset={strokeDashoffset} strokeLinecap="round"
                        transform="rotate(-90 50 50)" style={{ transition: "stroke-dashoffset 0.5s ease" }}
                      />
                    </svg>
                    <div className="doughnut-center-text">
                      <span className="doughnut-score">{productivityScore}%</span>
                      <span className="doughnut-sub">집중도</span>
                    </div>
                  </div>

                  <div className="stacked-progress-bar">
                    <div className="progress-segment productive" style={{ width: `${prodRatio}%` }} />
                    <div className="progress-segment unproductive" style={{ width: `${unprodRatio}%` }} />
                    <div className="progress-segment neutral" style={{ width: `${neutRatio}%` }} />
                  </div>

                  <div className="legend-container">
                    <div className="legend-item"><div className="legend-dot" style={{ backgroundColor: "var(--productive)" }} /><span>생산 ({Math.round(prodRatio)}%)</span></div>
                    <div className="legend-item"><div className="legend-dot" style={{ backgroundColor: "var(--unproductive)" }} /><span>비생산 ({Math.round(unprodRatio)}%)</span></div>
                    <div className="legend-item"><div className="legend-dot" style={{ backgroundColor: "var(--neutral)" }} /><span>중립 ({Math.round(neutRatio)}%)</span></div>
                  </div>
                </div>

                {/* 방문 TOP 5 랭커 */}
                <div className="card">
                  <h4 style={{ margin: "0 0 1rem 0", fontSize: "0.85rem", color: "var(--neutral)", textTransform: "uppercase" }}>가장 많이 방문한 곳 TOP 5</h4>
                  {topFiveDomains.length === 0 ? (
                    <div style={{ fontSize: "0.8rem", color: "var(--neutral)", textAlign: "center", padding: "1rem 0" }}>방문 데이터가 없습니다.</div>
                  ) : (
                    topFiveDomains.map((item, idx) => {
                      const widthPercent = (item.total_duration_sec / maxTimeValue) * 100;
                      return (
                        <div key={idx} className="top-domain-row">
                          <div className="top-domain-info">
                            <span className="top-domain-name" title={item.window_title}>{item.window_title}</span>
                            <span className="top-domain-time">{formatDuration(item.total_duration_sec)}</span>
                          </div>
                          <div className="top-domain-bar-bg">
                            <div className={`top-domain-bar-fill ${item.category.toLowerCase()}`} style={{ width: `${widthPercent}%` }} />
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>

              {/* 오른쪽 메인 테이블 영역 */}
              <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem", flex: 1, minWidth: 0 }}>
                {/* 24시간 일일 업무 타임라인 입체 시각화 카드 */}
                <div className="card" style={{ paddingBottom: "2.25rem" }}>
                  <div className="stat-label" style={{ marginBottom: "0.5rem" }}>24시간 집중 타임라인 시각화 (0시 - 23시)</div>
                  <div className="timeline-grid">
                    {Array.from({ length: 24 }).map((_, hour) => {
                      const hourLogs = timeline.filter((item) => item.hour === hour);
                      const totalHrSec = hourLogs.reduce((sum, item) => sum + item.duration, 0) || 1;
                      
                      const prodHr = hourLogs.find((i) => i.category === "PRODUCTIVE")?.duration || 0;
                      const unprodHr = hourLogs.find((i) => i.category === "UNPRODUCTIVE")?.duration || 0;
                      const neutHr = hourLogs.find((i) => i.category === "NEUTRAL")?.duration || 0;

                      return (
                        <div key={hour} className="timeline-hour-col">
                          <div className="timeline-bar-segment productive" style={{ height: `${(prodHr / totalHrSec) * 100}%` }} />
                          <div className="timeline-bar-segment unproductive" style={{ height: `${(unprodHr / totalHrSec) * 100}%` }} />
                          <div className="timeline-bar-segment neutral" style={{ height: `${(neutHr / totalHrSec) * 100}%` }} />
                          {hour % 2 === 0 && <span className="timeline-hour-label">{hour}시</span>}
                        </div>
                      );
                    })}
                  </div>
                </div>

                {/* 상세 내역 테이블 */}
                <div className="card" style={{ overflowX: "auto", margin: 0 }}>
                  <h3 style={{ marginBottom: "1rem" }}>{isGroupedByDomain ? "도메인별 요약 집계" : "상세 활성 탭 로그"}</h3>
                  <table>
                    <thead>
                      <tr>
                        <th>프로그램/웹사이트</th>
                        <th>{isGroupedByDomain ? "방문 도메인" : "상세 창 제목"}</th>
                        <th>분류 (클릭 시 수정)</th>
                        <th>총 사용 시간</th>
                        <th style={{ width: "40px" }}></th>
                      </tr>
                    </thead>
                    <tbody>
                      {displayReport.length === 0 ? (
                        <tr>
                          <td colSpan={5} style={{ textAlign: "center", color: "var(--neutral)" }}>기록된 통계가 아직 없습니다.</td>
                        </tr>
                      ) : (
                        displayReport.map((item, index) => (
                          <tr key={index}>
                            <td style={{ fontWeight: 600 }}>{item.process_name}</td>
                            <td className="window-title-cell">{item.window_title.trim() === "" ? "(제목 없음)" : item.window_title}</td>
                            <td>
                              <select
                                className={`badge-select ${item.category.toLowerCase()}`}
                                value={item.category}
                                onChange={(e) => handleInlineCategoryChange(item.window_title, item.category, e.target.value)}
                              >
                                <option value="PRODUCTIVE">생산적</option>
                                <option value="UNPRODUCTIVE">비생산적</option>
                                <option value="NEUTRAL">중립</option>
                                <option value="IGNORE">기록 제외</option>
                              </select>
                            </td>
                            <td>{formatDuration(item.total_duration_sec)}</td>
                            <td>
                              <button className="btn-danger-icon" onClick={() => setDeleteTarget(item)} title="기록 관리">✕</button>
                            </td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === "rules" && (
          <div>
            <div className="card">
              <h3>생산적/비생산적 키워드 규칙 추가</h3>
              <form onSubmit={handleAddRule}>
                <div className="form-group">
                  <div style={{ flex: 2 }}>
                    <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--neutral)" }}>매칭할 단어 (예: VS Code, YouTube)</label>
                    <input type="text" className="input-control" placeholder="키워드를 입력해 주세요" value={newPattern} onChange={(e) => setNewPattern(e.target.value)} />
                  </div>
                  <div>
                    <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--neutral)" }}>비교 방식</label>
                    <select className="input-control" value={newMatchType} onChange={(e) => setNewMatchType(e.target.value)}>
                      <option value="CONTAINS">단어 포함</option>
                      <option value="EQUALS">완전 일치</option>
                    </select>
                  </div>
                  <div>
                    <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--neutral)" }}>분류 유형</label>
                    <select className="input-control" value={newCategory} onChange={(e) => setNewCategory(e.target.value)}>
                      <option value="PRODUCTIVE">생산적</option>
                      <option value="UNPRODUCTIVE">비생산적</option>
                      <option value="IGNORE">기록 제외</option>
                    </select>
                  </div>
                  <button type="submit" className="btn-primary" style={{ height: "40px" }}>規則 추가</button>
                </div>
              </form>
            </div>

            <div className="card">
              <h3>현재 작동 중인 매칭 규칙</h3>
              <table>
                <thead>
                  <tr>
                    <th>매칭 키워드</th>
                    <th>매칭 기준</th>
                    <th>분류 지정</th>
                    <th>관리</th>
                  </tr>
                </thead>
                <tbody>
                  {rules.length === 0 ? (
                    <tr>
                      <td colSpan={4} style={{ textAlign: "center", color: "var(--neutral)" }}>등록된 자동 분류 규칙이 없습니다.</td>
                    </tr>
                  ) : (
                    rules.map((rule) => (
                      <tr key={rule.id}>
                        <td style={{ fontWeight: 600 }}>{rule.pattern}</td>
                        <td>{rule.match_type === "CONTAINS" ? "포함 매치" : "완전 일치"}</td>
                        <td>
                          <span className={`badge-select ${rule.category.toLowerCase()}`}>
                            {rule.category === "PRODUCTIVE" ? "생산적" : rule.category === "UNPRODUCTIVE" ? "비생산적" : "기록 제외"}
                          </span>
                        </td>
                        <td>
                          {rule.id && <button className="btn-danger" onClick={() => handleDeleteRule(rule.id!)}>삭제</button>}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {activeTab === "settings" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
            {/* 자동 시작 설정 */}
            <div className="card">
              <h3>윈도우 부팅 및 시작 제어</h3>
              <div className="switch-container" style={{ marginTop: "1rem" }}>
                <label className="switch">
                  <input type="checkbox" checked={isAutostart} onChange={handleToggleAutostart} />
                  <span className="slider"></span>
                </label>
                <div>
                  <strong style={{ display: "block" }}>윈도우 로그인 시 자동 실행</strong>
                  <span style={{ fontSize: "0.85rem", color: "var(--neutral)" }}>컴퓨터를 켤 때 Focus Tracker 프로그램을 백그라운드에 자동으로 등록하여 누락 없이 시간을 기록합니다.</span>
                </div>
              </div>
            </div>

            {/* 프로젝트 추가 및 태그 관리 패널 */}
            <div className="card">
              <h3>수동 업무 프로젝트 설정</h3>
              <div style={{ display: "flex", gap: "1rem", alignItems: "flex-end", marginTop: "1rem" }}>
                <div style={{ flex: 1 }}>
                  <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--neutral)" }}>신규 프로젝트 그룹명 추가</label>
                  <input type="text" className="input-control" placeholder="예: B 앱 성능 보완, 기획서 초안" value={newProjectName} onChange={(e) => setNewProjectName(e.target.value)} />
                </div>
                <button className="btn-primary" style={{ height: "40px" }} onClick={handleAddProject}>등록</button>
              </div>
            </div>

            {/* Gist 개인 클라우드 동기화 환경설정 패널 */}
            <div className="card">
              <h3>GitHub Gist 개인 클라우드 동기화</h3>
              <p style={{ fontSize: "0.85rem", color: "var(--neutral)", marginTop: "0.25rem" }}>
                개인의 보안이 유지되는 GitHub Gist 클라우드를 활용해 여러 장치에서 완벽하고 유실 없는 데이터 무손실 동기화를 가동합니다.
              </p>
              <div style={{ display: "flex", flexDirection: "column", gap: "1rem", marginTop: "1.25rem" }}>
                <div>
                  <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--neutral)" }}>GitHub Personal Access Token (PAT)</label>
                  <input
                    type="password"
                    className="input-control"
                    placeholder="ghp_로 시작하는 비공개 토큰을 입력해 주세요"
                    value={githubPat}
                    onChange={(e) => {
                      setGithubPat(e.target.value);
                      localStorage.setItem("github_pat", e.target.value);
                    }}
                  />
                </div>
                <div>
                  <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--neutral)" }}>Gist ID (최초 등록 시 생략 가능)</label>
                  <input
                    type="text"
                    className="input-control"
                    placeholder="최초 업로드 성공 시 자동으로 생성 및 업데이트됩니다"
                    value={gistId}
                    onChange={(e) => {
                      setGistId(e.target.value);
                      localStorage.setItem("gist_id", e.target.value);
                    }}
                  />
                </div>
                
                <div style={{ display: "flex", gap: "1rem", alignItems: "center", marginTop: "0.5rem" }}>
                  <button className="btn-primary" onClick={handleCloudBackup}>☁️ 클라우드로 즉시 업로드 백업</button>
                  <button className="btn-primary" style={{ backgroundColor: "var(--header-bg)", color: "var(--text)" }} onClick={handleCloudRestore}>🔄 클라우드 데이터 내려받아 동기화</button>
                </div>
                
                {syncStatus && (
                  <div style={{ fontSize: "0.85rem", fontWeight: 700, color: syncStatus.includes("성공") || syncStatus.includes("완료") ? "var(--productive)" : "var(--unproductive)" }}>
                    {syncStatus}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}
      </main>

      {/* 스마트 'X' 지능형 컨트롤 모달 레이어 */}
      {deleteTarget && (
        <div className="modal-backdrop">
          <div className="modal">
            <div className="modal-header">기록 관리 옵션 선택</div>
            <div className="modal-body">
              <strong>"{deleteTarget.process_name}"</strong>의 과거 모든 기록 데이터를 삭제합니다. <br />
              이 프로그램의 향후 수집에 대한 처리 방안을 선택해 주세요.
            </div>
            <div className="modal-footer">
              <button className="btn-block danger-solid" onClick={() => executeDeleteAndIgnore(false)}>이번 한 번만 과거 기록 전체 삭제</button>
              <button className="btn-block" style={{ backgroundColor: "var(--primary)", color: "var(--bg)", border: "none" }} onClick={() => executeDeleteAndIgnore(true)}>
                과거 기록 전체 삭제 + 향후 수집 완전 차단(Ignore)
              </button>
              <button className="btn-block secondary" onClick={() => setDeleteTarget(null)}>취소</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// 3. 동적 해시 라우팅 분기 엔트리포인트 컴포넌트
export default function App() {
  const [isWarning, setIsWarning] = useState<boolean>(() => {
    return window.location.hash === "#focus_warning";
  });

  useEffect(() => {
    const checkIsWarningLabel = async () => {
      try {
        const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const currentLabel = getCurrentWebviewWindow().label;
        if (currentLabel === "focus_warning") {
          setIsWarning(true); // 🌟 [오타 완전 박멸]: 이제 올바른 상태 변경자가 정확히 호출됩니다!
        }
      } catch (e) {
        // fallback
      }
    };
    checkIsWarningLabel();
  }, []);

  if (isWarning) {
    return <WarningApp />;
  }

  return <DashboardApp />;
}