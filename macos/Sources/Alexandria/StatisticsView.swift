import SwiftUI
import Charts

enum StatSection: String, CaseIterable, Identifiable {
    case overview = "Overview"
    case timeline = "Timeline"
    case activity = "Activity"
    case topSites = "Top Sites"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .overview: return "chart.bar.xaxis"
        case .timeline: return "calendar"
        case .activity: return "clock"
        case .topSites: return "globe"
        }
    }
}

struct StatisticsView: View {
    let engine: SearchEngineWrapper?

    @State private var selection: StatSection? = .overview
    @State private var isLoading = true

    // Data
    @State private var dailyCounts: [DailyPageCount] = []
    @State private var dailyBytes: [DailyPageCount] = []
    @State private var dayHourCells: [DayHourCell] = []
    @State private var topDomains: [TopDomain] = []
    @State private var summary: SummaryCounts?

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                ForEach(StatSection.allCases) { section in
                    Label(section.rawValue, systemImage: section.icon)
                        .tag(section)
                }
            }
            .listStyle(.sidebar)
            .navigationSplitViewColumnWidth(min: 160, ideal: 180)
        } detail: {
            if isLoading {
                ProgressView("Loading statistics...")
            } else {
                switch selection {
                case .overview:
                    OverviewPanel(dailyCounts: dailyCounts, summary: summary)
                case .timeline:
                    ContributionGraphPanel(dailyCounts: dailyCounts, dailyBytes: dailyBytes)
                case .activity:
                    ActivityHeatmapPanel(cells: dayHourCells)
                case .topSites:
                    TopSitesPanel(domains: topDomains)
                case nil:
                    Text("Select a section")
                        .foregroundColor(.secondary)
                }
            }
        }
        .onAppear { loadData() }
    }

    private func loadData() {
        let storePath = AppSettings.shared.storePath
        DispatchQueue.global(qos: .userInitiated).async {
            let dc = engine?.dailyPageCounts(storePath: storePath) ?? []
            let db = engine?.dailyByteCounts(storePath: storePath) ?? []
            let dh = engine?.dayHourBreakdown(storePath: storePath) ?? []
            let td = engine?.topDomains(storePath: storePath, limit: 50) ?? []
            let sc = engine?.summaryCounts(storePath: storePath)
            DispatchQueue.main.async {
                dailyCounts = dc
                dailyBytes = db
                dayHourCells = dh
                topDomains = td
                summary = sc
                isLoading = false
            }
        }
    }
}

// MARK: - Overview Panel

private struct OverviewPanel: View {
    let dailyCounts: [DailyPageCount]
    let summary: SummaryCounts?

    @State private var hoveredDate: Date?

    private static let displayFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateStyle = .medium
        return f
    }()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                summaryCards
                chartSection
            }
            .padding()
        }
    }

    private var summaryCards: some View {
        HStack(spacing: 12) {
            SummaryCard(label: "Total", value: summary?.total ?? 0)
            SummaryCard(label: "Today", value: summary?.today ?? 0)
            SummaryCard(label: "This Week", value: summary?.thisWeek ?? 0)
            SummaryCard(label: "This Month", value: summary?.thisMonth ?? 0)
            SummaryCard(label: "This Year", value: summary?.thisYear ?? 0)
        }
    }

    private var chartSection: some View {
        let recent = recentDailyDates(days: 90)
        return VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Pages per day (last 90 days)")
                    .font(.headline)
                Spacer()
                if let date = hoveredDate, let count = countFor(date: date, in: recent) {
                    Text("\(Self.displayFormatter.string(from: date)): \(count) page\(count == 1 ? "" : "s")")
                        .font(.callout)
                        .foregroundColor(.secondary)
                }
            }
            Chart(recent, id: \.0) { item in
                BarMark(
                    x: .value("Date", item.0, unit: .day),
                    y: .value("Pages", item.1)
                )
                .foregroundStyle(Color.accentColor)
            }
            .chartXAxis {
                AxisMarks(values: .stride(by: .month)) { _ in
                    AxisGridLine()
                    AxisValueLabel(format: .dateTime.month(.abbreviated).day())
                }
            }
            .chartYAxisLabel("Pages")
            .chartOverlay { proxy in
                GeometryReader { geo in
                    Rectangle().fill(.clear).contentShape(Rectangle())
                        .onContinuousHover { phase in
                            switch phase {
                            case .active(let location):
                                if let date: Date = proxy.value(atX: location.x) {
                                    hoveredDate = Calendar.current.startOfDay(for: date)
                                }
                            case .ended:
                                hoveredDate = nil
                            }
                        }
                }
            }
            .frame(height: 250)
        }
    }

    private func countFor(date: Date, in data: [(Date, Int64)]) -> Int64? {
        let cal = Calendar.current
        return data.first(where: { cal.isDate($0.0, inSameDayAs: date) })?.1
    }

    private func recentDailyDates(days: Int) -> [(Date, Int64)] {
        let cutoff = Calendar.current.date(byAdding: .day, value: -days, to: Date())!
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        let cutoffStr = formatter.string(from: cutoff)
        return dailyCounts
            .filter { $0.day >= cutoffStr }
            .compactMap { item in
                guard let date = formatter.date(from: item.day) else { return nil }
                return (date, item.count)
            }
    }
}

private struct SummaryCard: View {
    let label: String
    let value: Int64

    var body: some View {
        VStack(spacing: 4) {
            Text("\(value)")
                .font(.title)
                .fontWeight(.semibold)
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
        .background(RoundedRectangle(cornerRadius: 8).fill(.quaternary))
    }
}

// MARK: - Color Legend

private struct HeatmapLegend: View {
    let labels: [String]

    var body: some View {
        HStack(spacing: 4) {
            Text(labels[0])
                .font(.system(size: 9))
                .foregroundColor(.secondary)
            ForEach(Array(zip([0.0, 0.15, 0.4, 0.7, 1.0], labels.dropFirst())), id: \.0) { opacity, _ in
                RoundedRectangle(cornerRadius: 2)
                    .fill(opacity == 0 ? Color.secondary.opacity(0.1) : Color.accentColor.opacity(opacity))
                    .frame(width: 12, height: 12)
            }
            Text(labels[labels.count - 1])
                .font(.system(size: 9))
                .foregroundColor(.secondary)
        }
    }
}

// MARK: - Contribution Graph Panel

private struct ContributionGraphPanel: View {
    let dailyCounts: [DailyPageCount]
    let dailyBytes: [DailyPageCount]

    enum Metric: String, CaseIterable {
        case visits = "Visits"
        case bytes = "Content Size"
    }

    private let cellSize: CGFloat = 12
    private let spacing: CGFloat = 3
    private let dayLabels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]

    @State private var selectedMetric: Metric = .visits
    @State private var hoveredDate: String?
    @State private var hoveredValue: Int64 = 0

    var body: some View {
        let data = selectedMetric == .visits ? dailyCounts : dailyBytes
        let countMap = Dictionary(uniqueKeysWithValues: data.map { ($0.day, $0.count) })
        let grid = buildGrid()
        let thresholds = computeThresholds(countMap: countMap)

        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(selectedMetric == .visits ? "Pages per day" : "Content size per day")
                        .font(.headline)
                    Spacer()
                    Picker("Metric", selection: $selectedMetric) {
                        ForEach(Metric.allCases, id: \.self) { m in
                            Text(m.rawValue)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(width: 220)
                }

                if let date = hoveredDate {
                    Text("\(date): \(formattedHoverValue())")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(" ")
                        .font(.caption)
                }

                monthLabelsRow(grid: grid)

                HStack(alignment: .top, spacing: spacing) {
                    // Day labels
                    VStack(spacing: spacing) {
                        ForEach(0..<7, id: \.self) { row in
                            if row % 2 == 1 {
                                Text(dayLabels[row])
                                    .font(.system(size: 9))
                                    .foregroundColor(.secondary)
                                    .frame(width: 24, height: cellSize)
                            } else {
                                Text("")
                                    .frame(width: 24, height: cellSize)
                            }
                        }
                    }

                    // Grid
                    HStack(spacing: spacing) {
                        ForEach(0..<grid.count, id: \.self) { col in
                            VStack(spacing: spacing) {
                                ForEach(0..<grid[col].count, id: \.self) { row in
                                    let dateStr = grid[col][row]
                                    let count = countMap[dateStr] ?? 0
                                    RoundedRectangle(cornerRadius: 2)
                                        .fill(cellColor(count: count, thresholds: thresholds))
                                        .frame(width: cellSize, height: cellSize)
                                        .onHover { hovering in
                                            if hovering {
                                                hoveredDate = dateStr
                                                hoveredValue = count
                                            } else if hoveredDate == dateStr {
                                                hoveredDate = nil
                                            }
                                        }
                                }
                            }
                        }
                    }
                }

                HStack {
                    Spacer()
                    HeatmapLegend(labels: legendLabels(thresholds: thresholds))
                }
                .padding(.top, 4)
            }
            .padding()
        }
    }

    private func formattedHoverValue() -> String {
        if selectedMetric == .bytes {
            let formatter = ByteCountFormatter()
            formatter.countStyle = .file
            return formatter.string(fromByteCount: hoveredValue)
        }
        return "\(hoveredValue) page\(hoveredValue == 1 ? "" : "s")"
    }

    private func legendLabels(thresholds: [Int64]) -> [String] {
        if selectedMetric == .bytes {
            let fmt = ByteCountFormatter()
            fmt.countStyle = .file
            return ["0", fmt.string(fromByteCount: thresholds[0]), fmt.string(fromByteCount: thresholds[1]),
                    fmt.string(fromByteCount: thresholds[2]), fmt.string(fromByteCount: thresholds[3]) + "+"]
        }
        return ["0", "\(thresholds[0])", "\(thresholds[1])", "\(thresholds[2])", "\(thresholds[3])+"]
    }

    private func buildGrid() -> [[String]] {
        let calendar = Calendar.current
        let today = Date()
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"

        guard let startDate = calendar.date(byAdding: .day, value: -364, to: today) else { return [] }

        let startWeekday = calendar.component(.weekday, from: startDate)
        guard let adjustedStart = calendar.date(byAdding: .day, value: -(startWeekday - 1), to: startDate) else { return [] }

        var weeks: [[String]] = []
        var current = adjustedStart
        while current <= today {
            var week: [String] = []
            for _ in 0..<7 {
                week.append(formatter.string(from: current))
                current = calendar.date(byAdding: .day, value: 1, to: current)!
            }
            weeks.append(week)
        }
        return weeks
    }

    private func monthLabelsRow(grid: [[String]]) -> some View {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        let monthFormatter = DateFormatter()
        monthFormatter.dateFormat = "MMM"

        var labels: [(Int, String)] = []
        var lastMonth = -1
        for (i, week) in grid.enumerated() {
            if let date = formatter.date(from: week[0]) {
                let month = Calendar.current.component(.month, from: date)
                if month != lastMonth {
                    labels.append((i, monthFormatter.string(from: date)))
                    lastMonth = month
                }
            }
        }

        return HStack(spacing: 0) {
            Text("").frame(width: 24 + spacing)
            ZStack(alignment: .leading) {
                ForEach(labels, id: \.0) { (col, label) in
                    Text(label)
                        .font(.system(size: 9))
                        .foregroundColor(.secondary)
                        .offset(x: CGFloat(col) * (cellSize + spacing))
                }
            }
        }
        .frame(height: 14)
    }

    private func computeThresholds(countMap: [String: Int64]) -> [Int64] {
        let values = countMap.values.filter { $0 > 0 }.sorted()
        guard !values.isEmpty else { return [1, 2, 3, 4] }
        let p25 = values[values.count / 4]
        let p50 = values[values.count / 2]
        let p75 = values[values.count * 3 / 4]
        return [max(1, p25), max(p25 + 1, p50), max(p50 + 1, p75), max(p75 + 1, p75 + 1)]
    }

    private func cellColor(count: Int64, thresholds: [Int64]) -> Color {
        if count == 0 { return Color.secondary.opacity(0.1) }
        if count < thresholds[1] { return Color.accentColor.opacity(0.15) }
        if count < thresholds[2] { return Color.accentColor.opacity(0.4) }
        if count < thresholds[3] { return Color.accentColor.opacity(0.7) }
        return Color.accentColor
    }
}

// MARK: - Activity Heatmap Panel

private struct ActivityHeatmapPanel: View {
    let cells: [DayHourCell]

    enum Metric: String, CaseIterable {
        case visits = "Visits"
        case domains = "Domains"
        case bytes = "Bytes"
    }

    @State private var selectedMetric: Metric = .visits

    private let dayLabels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    private let cellSize: CGFloat = 22
    private let spacing: CGFloat = 2

    var body: some View {
        let lookup = buildLookup()
        let maxVal = maxValue(lookup: lookup)

        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("Activity by day of week and hour")
                        .font(.headline)
                    Spacer()
                    Picker("Metric", selection: $selectedMetric) {
                        ForEach(Metric.allCases, id: \.self) { m in
                            Text(m.rawValue)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(width: 300)
                }

                // Hour labels
                HStack(spacing: 0) {
                    Text("").frame(width: 36)
                    ForEach(0..<24, id: \.self) { hour in
                        if hour % 3 == 0 {
                            Text("\(hour)")
                                .font(.system(size: 9))
                                .foregroundColor(.secondary)
                                .frame(width: cellSize + spacing)
                        } else {
                            Text("")
                                .frame(width: cellSize + spacing)
                        }
                    }
                }

                // Grid rows
                ForEach(0..<7, id: \.self) { dow in
                    HStack(spacing: spacing) {
                        Text(dayLabels[dow])
                            .font(.system(size: 10))
                            .foregroundColor(.secondary)
                            .frame(width: 34, alignment: .trailing)

                        ForEach(0..<24, id: \.self) { hour in
                            let val = cellValue(dow: Int32(dow), hour: Int32(hour), lookup: lookup)
                            RoundedRectangle(cornerRadius: 3)
                                .fill(heatColor(value: val, max: maxVal))
                                .frame(width: cellSize, height: cellSize)
                                .help(tooltipText(dow: dow, hour: hour, val: val))
                        }
                    }
                }

                HStack {
                    Spacer()
                    activityLegend(maxVal: maxVal)
                }
                .padding(.top, 4)
            }
            .padding()
        }
    }

    private func activityLegend(maxVal: Int64) -> some View {
        let quarters = [0, maxVal / 4, maxVal / 2, maxVal * 3 / 4, maxVal]
        let labels: [String]
        if selectedMetric == .bytes {
            let fmt = ByteCountFormatter()
            fmt.countStyle = .file
            labels = ["0", fmt.string(fromByteCount: quarters[1]),
                      fmt.string(fromByteCount: quarters[2]),
                      fmt.string(fromByteCount: quarters[3]),
                      fmt.string(fromByteCount: quarters[4])]
        } else {
            let unit = selectedMetric == .visits ? "visits" : "domains"
            labels = ["0 \(unit)", "\(quarters[1])", "\(quarters[2])", "\(quarters[3])", "\(quarters[4])"]
        }
        return HeatmapLegend(labels: labels)
    }

    private func buildLookup() -> [Int64: DayHourCell] {
        var map: [Int64: DayHourCell] = [:]
        for cell in cells {
            let key = Int64(cell.dayOfWeek) * 100 + Int64(cell.hour)
            map[key] = cell
        }
        return map
    }

    private func cellValue(dow: Int32, hour: Int32, lookup: [Int64: DayHourCell]) -> Int64 {
        let key = Int64(dow) * 100 + Int64(hour)
        guard let cell = lookup[key] else { return 0 }
        switch selectedMetric {
        case .visits: return cell.visits
        case .domains: return cell.distinctDomains
        case .bytes: return cell.compressedBytes
        }
    }

    private func maxValue(lookup: [Int64: DayHourCell]) -> Int64 {
        var m: Int64 = 1
        for cell in lookup.values {
            let v: Int64
            switch selectedMetric {
            case .visits: v = cell.visits
            case .domains: v = cell.distinctDomains
            case .bytes: v = cell.compressedBytes
            }
            if v > m { m = v }
        }
        return m
    }

    private func heatColor(value: Int64, max: Int64) -> Color {
        if value == 0 { return Color.secondary.opacity(0.1) }
        let ratio = Double(value) / Double(max)
        if ratio < 0.25 { return Color.accentColor.opacity(0.15) }
        if ratio < 0.5 { return Color.accentColor.opacity(0.4) }
        if ratio < 0.75 { return Color.accentColor.opacity(0.7) }
        return Color.accentColor
    }

    private func tooltipText(dow: Int, hour: Int, val: Int64) -> String {
        let label: String
        switch selectedMetric {
        case .visits: label = "\(val) visit\(val == 1 ? "" : "s")"
        case .domains: label = "\(val) domain\(val == 1 ? "" : "s")"
        case .bytes:
            let formatter = ByteCountFormatter()
            formatter.countStyle = .file
            label = formatter.string(fromByteCount: val)
        }
        return "\(dayLabels[dow]) \(hour):00 — \(label)"
    }
}

// MARK: - Top Sites Panel

private struct TopSitesPanel: View {
    let domains: [TopDomain]

    enum SortBy: String, CaseIterable {
        case visits = "By Visits"
        case size = "By Content Size"
    }

    @State private var sortBy: SortBy = .visits

    var body: some View {
        let sorted = sortedDomains()
        let maxVal = sorted.first.map { sortBy == .visits ? $0.visitCount : $0.totalBytes } ?? 1

        VStack(alignment: .leading, spacing: 8) {
            Picker("Sort", selection: $sortBy) {
                ForEach(SortBy.allCases, id: \.self) { s in
                    Text(s.rawValue)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 300)
            .padding(.horizontal)
            .padding(.top, 8)

            List(Array(sorted.enumerated()), id: \.offset) { index, domain in
                HStack {
                    Text("\(index + 1)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .frame(width: 28, alignment: .trailing)

                    Text(domain.domain)
                        .frame(width: 200, alignment: .leading)
                        .lineLimit(1)

                    GeometryReader { geo in
                        let val = sortBy == .visits ? domain.visitCount : domain.totalBytes
                        let width = geo.size.width * CGFloat(val) / CGFloat(max(maxVal, 1))
                        RoundedRectangle(cornerRadius: 3)
                            .fill(Color.accentColor.opacity(0.6))
                            .frame(width: max(width, 2), height: 16)
                            .frame(maxHeight: .infinity, alignment: .center)
                    }

                    Spacer()

                    Text(formattedValue(domain: domain))
                        .font(.callout)
                        .foregroundColor(.secondary)
                        .frame(width: 80, alignment: .trailing)
                }
                .frame(height: 24)
            }
        }
    }

    private func sortedDomains() -> [TopDomain] {
        switch sortBy {
        case .visits:
            return domains.sorted { $0.visitCount > $1.visitCount }
        case .size:
            return domains.sorted { $0.totalBytes > $1.totalBytes }
        }
    }

    private func formattedValue(domain: TopDomain) -> String {
        switch sortBy {
        case .visits:
            return NumberFormatter.localizedString(from: NSNumber(value: domain.visitCount), number: .decimal)
        case .size:
            let formatter = ByteCountFormatter()
            formatter.countStyle = .file
            return formatter.string(fromByteCount: domain.totalBytes)
        }
    }
}
