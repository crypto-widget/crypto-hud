#include "taskbar_protocol.h"

#include <windows.h>
#include <ctxtcall.h>
#include <ocidl.h>
#include <xamlom.h>

// winuser.h defines a legacy macro that conflicts with a generated
// C++/WinRT animation method.
#ifdef GetCurrentTime
#undef GetCurrentTime
#endif

#include <winrt/base.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.UI.h>
#include <winrt/Windows.UI.Core.h>
#include <winrt/Windows.UI.Xaml.h>
#include <winrt/Windows.UI.Xaml.Automation.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Input.h>
#include <winrt/Windows.UI.Xaml.Media.h>

#include <algorithm>
#include <atomic>
#include <cstdint>
#include <cwchar>
#include <mutex>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace crypto_hud::taskbar {
namespace {

using winrt::Windows::Foundation::IInspectable;
using winrt::Windows::UI::Color;
using winrt::Windows::UI::Core::CoreDispatcher;
using winrt::Windows::UI::Core::CoreDispatcherPriority;
using winrt::Windows::UI::Xaml::DependencyObject;
using winrt::Windows::UI::Xaml::FlowDirection;
using winrt::Windows::UI::Xaml::FrameworkElement;
using winrt::Windows::UI::Xaml::GridLengthHelper;
using winrt::Windows::UI::Xaml::HorizontalAlignment;
using winrt::Windows::UI::Xaml::TextAlignment;
using winrt::Windows::UI::Xaml::TextTrimming;
using winrt::Windows::UI::Xaml::TextWrapping;
using winrt::Windows::UI::Xaml::Thickness;
using winrt::Windows::UI::Xaml::VerticalAlignment;
using winrt::Windows::UI::Xaml::Automation::AutomationProperties;
using winrt::Windows::UI::Xaml::Controls::ColumnDefinition;
using winrt::Windows::UI::Xaml::Controls::Control;
using winrt::Windows::UI::Xaml::Controls::Grid;
using winrt::Windows::UI::Xaml::Controls::RowDefinition;
using winrt::Windows::UI::Xaml::Controls::TextBlock;
using winrt::Windows::UI::Xaml::Controls::ToolTipService;
using winrt::Windows::UI::Xaml::Input::TappedRoutedEventArgs;
using winrt::Windows::UI::Xaml::Input::RightTappedRoutedEventArgs;
using winrt::Windows::UI::Xaml::Media::Brush;
using winrt::Windows::UI::Xaml::Media::SolidColorBrush;
using winrt::Windows::UI::Xaml::Media::VisualTreeHelper;

constexpr CLSID kTapClsid{
    0x2304531e,
    0xb59e,
    0x4f0e,
    {0xb3, 0xa7, 0x80, 0x53, 0x50, 0x05, 0x07, 0x6a},
};

constexpr wchar_t kTaskbarGridName[] = L"SystemTrayFrameGrid";
constexpr wchar_t kClockName[] = L"NotificationCenterButton";
constexpr wchar_t kMarketRootName[] = L"CryptoHudTaskbarMarketV6";
constexpr wchar_t kSlintTrayWindowClass[] = L"SlintSystemTrayWindow";
constexpr UINT kSlintTrayCallbackMessage = WM_APP + 1;
constexpr WPARAM kSlintTrayUid = 1;
constexpr std::uint64_t kHeartbeatTimeoutMilliseconds = 5000;

struct MarketSnapshot {
    std::uint32_t owner_pid{};
    std::uint64_t heartbeat_ms{};
    bool enabled{};
    std::wstring symbol;
    std::wstring price;
    std::wstring tooltip;
    std::uint32_t accent_argb{};
};

std::mutex g_mapping_mutex;
HANDLE g_mapping = nullptr;
SharedMarketState* g_shared_state = nullptr;
std::atomic<bool> g_xaml_initialization_succeeded{false};
std::atomic<bool> g_xaml_initialization_active{false};

template <std::size_t Size>
std::wstring CopyFixedString(const wchar_t (&source)[Size]) {
    std::size_t length = 0;
    while (length < Size && source[length] != L'\0') {
        ++length;
    }
    return std::wstring{source, source + length};
}

bool EnsureSharedState() noexcept {
    if (g_shared_state != nullptr) {
        return true;
    }

    std::scoped_lock lock{g_mapping_mutex};
    if (g_shared_state != nullptr) {
        return true;
    }

    HANDLE mapping = OpenFileMappingW(
        FILE_MAP_READ | FILE_MAP_WRITE, FALSE, kMappingName);
    if (mapping == nullptr) {
        return false;
    }

    void* view = MapViewOfFile(
        mapping, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0,
        sizeof(SharedMarketState));
    if (view == nullptr) {
        CloseHandle(mapping);
        return false;
    }

    auto* state = static_cast<SharedMarketState*>(view);
    if (state->magic != kProtocolMagic ||
        state->version != kProtocolVersion ||
        state->size != sizeof(SharedMarketState)) {
        UnmapViewOfFile(view);
        CloseHandle(mapping);
        return false;
    }

    g_mapping = mapping;
    g_shared_state = state;
    return true;
}

bool ReadSnapshot(MarketSnapshot& output) noexcept {
    if (!EnsureSharedState()) {
        return false;
    }

    for (int attempt = 0; attempt < 8; ++attempt) {
        const LONG before = InterlockedCompareExchange(
            &g_shared_state->sequence, 0, 0);
        if ((before & 1) != 0) {
            YieldProcessor();
            continue;
        }

        const std::uint32_t owner_pid = g_shared_state->owner_pid;
        const std::uint64_t heartbeat_ms = g_shared_state->heartbeat_ms;
        const bool enabled = g_shared_state->enabled != 0;
        wchar_t symbol[std::size(g_shared_state->symbol)]{};
        wchar_t price[std::size(g_shared_state->price)]{};
        wchar_t tooltip[std::size(g_shared_state->tooltip)]{};
        std::copy_n(g_shared_state->symbol, std::size(symbol), symbol);
        std::copy_n(g_shared_state->price, std::size(price), price);
        std::copy_n(g_shared_state->tooltip, std::size(tooltip), tooltip);
        const std::uint32_t accent_argb = g_shared_state->accent_argb;

        MemoryBarrier();
        const LONG after = InterlockedCompareExchange(
            &g_shared_state->sequence, 0, 0);
        if (before == after && (after & 1) == 0) {
            output.owner_pid = owner_pid;
            output.heartbeat_ms = heartbeat_ms;
            output.enabled = enabled;
            output.symbol = CopyFixedString(symbol);
            output.price = CopyFixedString(price);
            output.tooltip = CopyFixedString(tooltip);
            output.accent_argb = accent_argb;
            return true;
        }
    }

    return false;
}

std::uint32_t ReadOwnerPidBestEffort() noexcept {
    if (!EnsureSharedState()) {
        return 0;
    }
    const LONG owner_pid = InterlockedCompareExchange(
        reinterpret_cast<volatile LONG*>(&g_shared_state->owner_pid), 0, 0);
    return static_cast<std::uint32_t>(owner_pid);
}

bool IsOwnerProcessAlive(std::uint32_t owner_pid) noexcept {
    if (owner_pid == 0) {
        return false;
    }

    HANDLE process = OpenProcess(
        SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
        FALSE,
        owner_pid);
    if (process == nullptr) {
        // Access can be denied for a protected/elevated host. A fresh
        // heartbeat is stronger evidence than treating that as a dead PID.
        return GetLastError() == ERROR_ACCESS_DENIED;
    }
    const DWORD wait_result = WaitForSingleObject(process, 0);
    CloseHandle(process);
    return wait_result == WAIT_TIMEOUT;
}

bool IsOwnerHealthy(MarketSnapshot const& snapshot) noexcept {
    if (snapshot.heartbeat_ms == 0 ||
        GetTickCount64() - snapshot.heartbeat_ms >
            kHeartbeatTimeoutMilliseconds) {
        return false;
    }
    return IsOwnerProcessAlive(snapshot.owner_pid);
}

void SetStatus(TaskbarStatus status, HRESULT error = S_OK) noexcept {
    if (!EnsureSharedState()) {
        return;
    }
    InterlockedExchange(
        &g_shared_state->status, static_cast<LONG>(status));
    InterlockedExchange(&g_shared_state->error_code, error);
    InterlockedExchange(
        &g_shared_state->explorer_pid, static_cast<LONG>(GetCurrentProcessId()));
}

void RecordAction() noexcept {
    if (EnsureSharedState()) {
        InterlockedIncrement(&g_shared_state->action_sequence);
    }

    HANDLE event = OpenEventW(EVENT_MODIFY_STATE, FALSE, kActionEventName);
    if (event != nullptr) {
        SetEvent(event);
        CloseHandle(event);
    }
}

bool IsNamed(FrameworkElement const& element, std::wstring_view name) {
    return element && std::wstring_view{element.Name()} == name;
}

bool HasClockClassName(DependencyObject const& object) {
    if (!object) {
        return false;
    }
    const auto class_name = winrt::get_class_name(object);
    return std::wstring_view{class_name}.find(kClockName) !=
        std::wstring_view::npos;
}

DependencyObject FindClock(DependencyObject const& root, unsigned depth = 0) {
    if (!root || depth > 32) {
        return nullptr;
    }

    if (auto element = root.try_as<FrameworkElement>();
        (element && IsNamed(element, kClockName)) || HasClockClassName(root)) {
        return root;
    }

    const int count = VisualTreeHelper::GetChildrenCount(root);
    for (int index = 0; index < count; ++index) {
        if (auto result = FindClock(
                VisualTreeHelper::GetChild(root, index), depth + 1)) {
            return result;
        }
    }
    return nullptr;
}

FrameworkElement FindNamedElement(
    DependencyObject const& root,
    std::wstring_view name,
    unsigned depth = 0) {
    if (!root || depth > 32) {
        return nullptr;
    }
    if (auto element = root.try_as<FrameworkElement>();
        element && IsNamed(element, name)) {
        return element;
    }

    const int count = VisualTreeHelper::GetChildrenCount(root);
    for (int index = 0; index < count; ++index) {
        if (auto result = FindNamedElement(
                VisualTreeHelper::GetChild(root, index), name, depth + 1)) {
            return result;
        }
    }
    return nullptr;
}

DependencyObject FindDirectClockChild(Grid const& parent) {
    // Search each direct child independently. This avoids relying on COM
    // interface-pointer equality while still returning the column anchor that
    // owns the clock's subtree.
    const int count = VisualTreeHelper::GetChildrenCount(parent);
    for (int index = 0; index < count; ++index) {
        auto child = VisualTreeHelper::GetChild(parent, index);
        if (child && FindClock(child)) {
            return child;
        }
    }
    return nullptr;
}

struct Candidate {
    InstanceHandle handle{};
    winrt::weak_ref<Grid> grid;
};

struct Attachment {
    InstanceHandle parent_handle{};
    winrt::weak_ref<Grid> parent;
    Grid root{nullptr};
    ColumnDefinition column{nullptr};
    TextBlock symbol{nullptr};
    TextBlock price{nullptr};
    TextBlock change{nullptr};
    Grid price_row{nullptr};
    Brush inherited_foreground{nullptr};
    winrt::event_token tapped_token{};
    winrt::event_token right_tapped_token{};
};

thread_local bool g_mutating_visual_tree = false;

#ifdef CRYPTO_HUD_TASKBAR_DEBUG
void DebugLine(std::wstring_view message) noexcept {
    static std::mutex debug_mutex;
    static unsigned debug_count = 0;
    std::scoped_lock lock{debug_mutex};
    if (debug_count++ >= 16000) {
        return;
    }

    wchar_t temporary_path[MAX_PATH]{};
    if (GetTempPathW(static_cast<DWORD>(std::size(temporary_path)),
                     temporary_path) == 0) {
        return;
    }
    const std::wstring path =
        std::wstring{temporary_path} + L"crypto-hud-taskbar-visual-tree.log";
    HANDLE file = CreateFileW(
        path.c_str(), FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE,
        nullptr, OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) {
        return;
    }

    DWORD bytes_written = 0;
    WriteFile(file, message.data(),
              static_cast<DWORD>(message.size() * sizeof(wchar_t)),
              &bytes_written, nullptr);
    constexpr wchar_t newline[] = L"\r\n";
    WriteFile(file, newline, 2 * sizeof(wchar_t), &bytes_written, nullptr);
    CloseHandle(file);
}

void DebugVisualElement(
    ParentChildRelation const& relation,
    VisualElement const& visual,
    VisualMutationType mutation_type) noexcept {
    const wchar_t* name = visual.Name != nullptr ? visual.Name : L"";
    const wchar_t* type = visual.Type != nullptr ? visual.Type : L"";
    wchar_t line[1024]{};
    const int characters = _snwprintf_s(
        line, _TRUNCATE, L"tree\t%u\t%llu\t%llu\t%llu\t%u\t%s\t%s\r\n",
        static_cast<unsigned>(mutation_type),
        static_cast<unsigned long long>(visual.Handle),
        static_cast<unsigned long long>(relation.Parent),
        static_cast<unsigned long long>(relation.Child),
        visual.NumChildren, name, type);
    if (characters > 0) {
        // DebugLine appends the newline itself.
        DebugLine(std::wstring_view{line, static_cast<std::size_t>(characters - 2)});
    }
}
#endif

bool ShowHostTrayContextMenu() noexcept {
    const std::uint32_t owner_pid = ReadOwnerPidBestEffort();
    if (!IsOwnerProcessAlive(owner_pid)) {
        return false;
    }

    HWND previous = nullptr;
    for (;;) {
        HWND window = FindWindowExW(
            HWND_MESSAGE, previous, kSlintTrayWindowClass, nullptr);
        if (window == nullptr) {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"action\tcontext-menu-window-missing");
#endif
            return false;
        }

        DWORD window_pid = 0;
        GetWindowThreadProcessId(window, &window_pid);
        if (window_pid == owner_pid &&
            PostMessageW(
                window,
                kSlintTrayCallbackMessage,
                kSlintTrayUid,
                static_cast<LPARAM>(WM_CONTEXTMENU))) {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"action\tcontext-menu-posted");
#endif
            return true;
        }
        previous = window;
    }
}

class MutationScope {
public:
    MutationScope() noexcept : previous_{g_mutating_visual_tree} {
        g_mutating_visual_tree = true;
    }

    ~MutationScope() { g_mutating_visual_tree = previous_; }

private:
    bool previous_;
};

class AtomicActivityScope {
public:
    explicit AtomicActivityScope(std::atomic<unsigned>& active) noexcept
        : active_{active} {
        active_.fetch_add(1, std::memory_order_acq_rel);
    }

    ~AtomicActivityScope() {
        active_.fetch_sub(1, std::memory_order_acq_rel);
    }

private:
    std::atomic<unsigned>& active_;
};

class VisualTreeWatcher
    : public winrt::implements<
          VisualTreeWatcher,
          IVisualTreeServiceCallback2,
          winrt::non_agile> {
public:
    VisualTreeWatcher(
        winrt::com_ptr<IXamlDiagnostics> diagnostics,
        winrt::com_ptr<IVisualTreeService3> visual_tree_service)
        : diagnostics_{std::move(diagnostics)},
          visual_tree_service_{std::move(visual_tree_service)} {
        MarketSnapshot snapshot;
        if (ReadSnapshot(snapshot)) {
            enabled_ = snapshot.enabled && IsOwnerHealthy(snapshot);
            last_owner_pid_.store(snapshot.owner_pid, std::memory_order_release);
            latest_snapshot_ = std::move(snapshot);
        } else {
            last_owner_pid_.store(
                ReadOwnerPidBestEffort(), std::memory_order_release);
        }
        stop_event_ = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        stop_request_event_ = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        worker_ready_event_ = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        worker_start_event_ = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (stop_event_ == nullptr || stop_request_event_ == nullptr ||
            worker_ready_event_ == nullptr || worker_start_event_ == nullptr) {
            const DWORD error = GetLastError();
            event_error_ = HRESULT_FROM_WIN32(
                error == ERROR_SUCCESS ? ERROR_NOT_ENOUGH_MEMORY : error);
        }
    }

    ~VisualTreeWatcher() {
        for (HANDLE event : {stop_event_, stop_request_event_,
                             worker_ready_event_, worker_start_event_}) {
            if (event != nullptr) {
                CloseHandle(event);
            }
        }
    }

    HRESULT Start() noexcept {
        if (FAILED(event_error_)) {
            SetStatus(TaskbarStatus::Error, event_error_);
            return event_error_;
        }

        HRESULT context_result = CoGetObjectContext(
            __uuidof(IContextCallback), advise_context_.put_void());
        if (FAILED(context_result)) {
            SetStatus(TaskbarStatus::Error, context_result);
            return context_result;
        }
        context_result = CoGetContextToken(&advise_context_token_);
        if (FAILED(context_result)) {
            advise_context_ = nullptr;
            SetStatus(TaskbarStatus::Error, context_result);
            return context_result;
        }

        // Start and initialize the retry owner before subscribing. If the
        // native thread cannot initialize COM, no callback is left advised.
        AddRef();
        HANDLE update_thread = CreateThread(
            nullptr, 0, &VisualTreeWatcher::UpdateThreadEntry, this, 0, nullptr);
        if (update_thread == nullptr) {
            const HRESULT error = HRESULT_FROM_WIN32(GetLastError());
            Release();
            SetStatus(TaskbarStatus::Error, error);
            return error;
        }
        CloseHandle(update_thread);

        const DWORD ready_result =
            WaitForSingleObject(worker_ready_event_, INFINITE);
        if (ready_result != WAIT_OBJECT_0) {
            const HRESULT error = ready_result == WAIT_FAILED
                ? HRESULT_FROM_WIN32(GetLastError())
                : E_UNEXPECTED;
            stop_requested_.store(true, std::memory_order_release);
            stopped_.store(true, std::memory_order_release);
            visual_tree_service_ = nullptr;
            diagnostics_ = nullptr;
            SetEvent(stop_event_);
            SetEvent(worker_start_event_);
            SetStatus(TaskbarStatus::Error, error);
            return error;
        }
        const HRESULT apartment_result =
            worker_apartment_result_.load(std::memory_order_acquire);
        if (FAILED(apartment_result)) {
            stop_requested_.store(true, std::memory_order_release);
            stopped_.store(true, std::memory_order_release);
            visual_tree_service_ = nullptr;
            diagnostics_ = nullptr;
            SetEvent(stop_event_);
            SetEvent(worker_start_event_);
            SetStatus(TaskbarStatus::Error, apartment_result);
            return apartment_result;
        }

        SetStatus(TaskbarStatus::WaitingForVisualTree);

        HRESULT advise_result = E_FAIL;
        try {
            advise_result = visual_tree_service_->AdviseVisualTreeChange(this);
        } catch (...) {
            advise_result = winrt::to_hresult();
        }
        if (FAILED(advise_result)) {
            stop_requested_.store(true, std::memory_order_release);
            stopped_.store(true, std::memory_order_release);
            visual_tree_service_ = nullptr;
            diagnostics_ = nullptr;
            SetEvent(stop_event_);
            SetEvent(worker_start_event_);
            SetStatus(TaskbarStatus::Error, advise_result);
            return advise_result;
        }
        advised_.store(true, std::memory_order_release);
        SetEvent(worker_start_event_);
        return stop_requested_.load(std::memory_order_acquire)
            ? HRESULT_FROM_WIN32(ERROR_CANCELLED)
            : S_OK;
    }

    HRESULT Stop() noexcept {
        const bool first_request =
            !stop_requested_.exchange(true, std::memory_order_acq_rel);
        if (first_request) {
            detach_attempts_.store(0, std::memory_order_release);
            QueueDetach();
            if (stop_request_event_ != nullptr) {
                SetEvent(stop_request_event_);
            }
        }
        return S_OK;
    }

    void Refresh() noexcept {
        AtomicActivityScope activity{active_diagnostics_users_};
        try {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"refresh\tentered");
#endif
            if (stop_requested_.load(std::memory_order_acquire)) {
                std::scoped_lock lock{mutex_};
                DetachAll(false);
                return;
            }
            MarketSnapshot snapshot;
            if (!ReadSnapshot(snapshot)) {
                return;
            }
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"refresh\tsnapshot");
#endif

            std::scoped_lock lock{mutex_};
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"refresh\tlocked");
#endif
            latest_snapshot_ = snapshot;
            enabled_ = snapshot.enabled && IsOwnerHealthy(snapshot);
            if (!enabled_) {
                DetachAll();
                return;
            }
            if (detach_pending_.load(std::memory_order_acquire)) {
                DetachAll();
                if (!attachments_.empty()) {
                    return;
                }
            }

#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"refresh\tattaching");
#endif
            const bool market_present = TryAttachAll(snapshot);
            UpdateAll(snapshot);
            SetStatus(market_present
                          ? TaskbarStatus::Attached
                          : TaskbarStatus::WaitingForVisualTree);
        } catch (...) {
            const HRESULT error = winrt::to_hresult();
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            wchar_t line[96]{};
            _snwprintf_s(line, _TRUNCATE,
                         L"refresh-error\t0x%08X", error);
            DebugLine(line);
#endif
            SetStatus(TaskbarStatus::Error, error);
        }
    }

    HRESULT STDMETHODCALLTYPE OnVisualTreeChange(
        ParentChildRelation relation,
        VisualElement element,
        VisualMutationType mutation_type) noexcept override {
        AtomicActivityScope activity{active_diagnostics_users_};
        if (g_mutating_visual_tree ||
            stop_requested_.load(std::memory_order_acquire)) {
            return S_OK;
        }

#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugVisualElement(relation, element, mutation_type);
#endif

        try {
            bool should_refresh = false;
            {
                std::scoped_lock lock{mutex_};
                if (stop_requested_.load(std::memory_order_acquire)) {
                    return S_OK;
                }
                if (mutation_type == VisualMutationType::Remove) {
                    ForgetHandle(element.Handle);
                    ForgetHandle(relation.Child);
                    return S_OK;
                }

                should_refresh = RememberCandidate(element) ||
                    (!candidates_.empty() && enabled_);
            }
            if (should_refresh) {
                // Always mutate the taskbar from its XAML dispatcher. Visual
                // tree callbacks can be delivered by the diagnostics thread.
                QueueRefresh();
            }
        } catch (...) {
            const HRESULT error = winrt::to_hresult();
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            wchar_t line[96]{};
            _snwprintf_s(line, _TRUNCATE,
                         L"callback-error\t0x%08X", error);
            DebugLine(line);
#endif
            SetStatus(TaskbarStatus::Error, error);
        }
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE OnElementStateChanged(
        InstanceHandle,
        VisualElementState,
        LPCWSTR) noexcept override {
        return S_OK;
    }

private:
    static DWORD WINAPI UpdateThreadEntry(void* parameter) noexcept {
        auto* self = static_cast<VisualTreeWatcher*>(parameter);
        const HRESULT apartment_result =
            CoInitializeEx(nullptr, COINIT_MULTITHREADED);
        self->worker_apartment_result_.store(
            apartment_result, std::memory_order_release);
        SetEvent(self->worker_ready_event_);
        WaitForSingleObject(self->worker_start_event_, INFINITE);
        if (SUCCEEDED(apartment_result)) {
            self->RunUpdateLoop();
        }
        // Drop the worker's self-reference while its COM apartment is still
        // initialized. This can run the final destructor and release the
        // captured context object, so nothing may access self after this line.
        self->Release();
        if (SUCCEEDED(apartment_result)) {
            CoUninitialize();
        }
        return 0;
    }

    static HRESULT __stdcall StopContextCallback(
        ComCallData* call_data) noexcept {
        if (call_data == nullptr || call_data->pUserDefined == nullptr) {
            return E_POINTER;
        }
        return static_cast<VisualTreeWatcher*>(call_data->pUserDefined)
            ->StopInAdviseContext();
    }

    HRESULT TryCompleteStop() noexcept {
        if (stopped_.load(std::memory_order_acquire)) {
            return S_OK;
        }

        ULONG_PTR current_context_token = 0;
        if (SUCCEEDED(CoGetContextToken(&current_context_token)) &&
            current_context_token == advise_context_token_) {
            return StopInAdviseContext();
        }
        if (!advise_context_) {
            const HRESULT error = CO_E_NOTINITIALIZED;
            SetStatus(TaskbarStatus::Error, error);
            return error;
        }

        ComCallData call_data{};
        call_data.pUserDefined = this;
        HRESULT result = E_FAIL;
        try {
            result = advise_context_->ContextCallback(
                &VisualTreeWatcher::StopContextCallback,
                &call_data,
                IID_ICallbackWithNoReentrancyToApplicationSTA,
                5,
                nullptr);
        } catch (...) {
            result = winrt::to_hresult();
        }
        if (stopped_.load(std::memory_order_acquire)) {
            return S_OK;
        }
        if (SUCCEEDED(result)) {
            result = E_UNEXPECTED;
        }
        // Keep advised_ set so this sole retry owner can try the captured COM
        // context again instead of orphaning the diagnostics callback. Busy is
        // an expected barrier while a pre-stop dispatcher refresh finishes.
        if (result != HRESULT_FROM_WIN32(ERROR_BUSY)) {
            SetStatus(TaskbarStatus::Detaching, result);
        }
        return result;
    }

    HRESULT StopInAdviseContext() noexcept {
        if (!stop_requested_.load(std::memory_order_acquire)) {
            return S_FALSE;
        }
        if (advised_.load(std::memory_order_acquire)) {
            HRESULT unadvise_result = E_FAIL;
            try {
                unadvise_result =
                    visual_tree_service_->UnadviseVisualTreeChange(this);
            } catch (...) {
                unadvise_result = winrt::to_hresult();
            }
            if (FAILED(unadvise_result)) {
                // Preserve the advised state. A later heartbeat pass or site
                // transition can retry in this same COM context.
                SetStatus(TaskbarStatus::Detaching, unadvise_result);
                return unadvise_result;
            }
            advised_.store(false, std::memory_order_release);
        }

        if (active_diagnostics_users_.load(std::memory_order_acquire) != 0) {
            // A dispatcher callback that started before Stop was requested can
            // still be finishing a diagnostics lookup. Never release its COM
            // services out from under it; the sole worker retries shortly.
            return HRESULT_FROM_WIN32(ERROR_BUSY);
        }
        visual_tree_service_ = nullptr;
        diagnostics_ = nullptr;

        if (stopped_.exchange(true, std::memory_order_acq_rel)) {
            return S_OK;
        }
        if (stop_event_ != nullptr) {
            SetEvent(stop_event_);
        }
        detach_attempts_.store(0, std::memory_order_release);
        QueueDetach();
        return S_OK;
    }

    void RunUpdateLoop() noexcept {
        HANDLE update_event = nullptr;
        for (;;) {
            if (stop_requested_.load(std::memory_order_acquire) &&
                SUCCEEDED(TryCompleteStop())) {
                break;
            }
            if (update_event == nullptr) {
                update_event = OpenEventW(
                    SYNCHRONIZE, FALSE, kUpdateEventName);
            }

            MarketSnapshot snapshot;
            const bool has_snapshot = ReadSnapshot(snapshot);
            const std::uint32_t observed_owner_pid = has_snapshot
                ? snapshot.owner_pid
                : ReadOwnerPidBestEffort();
            if (observed_owner_pid != 0) {
                last_owner_pid_.store(
                    observed_owner_pid, std::memory_order_release);
            }
            const std::uint32_t owner_pid = observed_owner_pid != 0
                ? observed_owner_pid
                : last_owner_pid_.load(std::memory_order_acquire);
            const bool owner_present = owner_pid != 0;
            const bool owner_alive = owner_present &&
                IsOwnerProcessAlive(owner_pid);
            if (owner_present && !owner_alive) {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
                if (!stop_requested_.load(std::memory_order_acquire)) {
                    DebugLine(L"watcher\towner-stale");
                }
#endif
                // Release the diagnostics callback when the host disappears.
                // Otherwise a versioned watcher can stay advised forever while
                // waiting on an event that no future protocol will signal.
                if (!stop_requested_.load(std::memory_order_acquire)) {
                    Stop();
                }
            }
            const DWORD timeout =
                (stop_requested_.load(std::memory_order_acquire) ||
                 owner_present ||
                 detach_pending_.load(std::memory_order_acquire))
                ? 1000
                : INFINITE;
            DWORD wait_result = WAIT_TIMEOUT;
            if (update_event != nullptr) {
                HANDLE handles[]{
                    stop_event_, stop_request_event_, update_event};
                wait_result = WaitForMultipleObjects(
                    static_cast<DWORD>(std::size(handles)),
                    handles,
                    FALSE,
                    timeout);
                if (wait_result == WAIT_FAILED) {
                    CloseHandle(update_event);
                    update_event = nullptr;
                    continue;
                }
                if (wait_result == WAIT_OBJECT_0) {
                    break;
                }
                if (wait_result == WAIT_OBJECT_0 + 1) {
                    continue;
                }
            } else {
                HANDLE handles[]{stop_event_, stop_request_event_};
                wait_result = WaitForMultipleObjects(
                    static_cast<DWORD>(std::size(handles)),
                    handles,
                    FALSE,
                    timeout);
                if (wait_result == WAIT_FAILED) {
                    continue;
                }
                if (wait_result == WAIT_OBJECT_0) {
                    break;
                }
                if (wait_result == WAIT_OBJECT_0 + 1) {
                    continue;
                }
            }

            // The timeout is intentional: it detects a departed owner even
            // after its final disabled frame has consumed the named event.
            QueueRefresh();
        }

        if (update_event != nullptr) {
            CloseHandle(update_event);
        }
    }

    void SetDispatcher(CoreDispatcher const& dispatcher) {
        std::scoped_lock lock{dispatcher_mutex_};
        dispatcher_ = dispatcher;
    }

    void QueueRefresh() noexcept {
        if (stop_requested_.load(std::memory_order_acquire)) {
            return;
        }
        CoreDispatcher dispatcher{nullptr};
        {
            std::scoped_lock lock{dispatcher_mutex_};
            dispatcher = dispatcher_;
        }
        if (!dispatcher || refresh_pending_.exchange(true)) {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(!dispatcher
                          ? L"refresh\tno-dispatcher"
                          : L"refresh\talready-pending");
#endif
            return;
        }

        AddRef();
        try {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"refresh\tqueued");
#endif
            dispatcher.RunAsync(
                CoreDispatcherPriority::Normal,
                [this]() noexcept {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
                    DebugLine(L"refresh\trunning");
#endif
                    refresh_pending_.store(false, std::memory_order_release);
                    Refresh();
                    Release();
                });
        } catch (...) {
            refresh_pending_.store(false, std::memory_order_release);
            Release();
            SetStatus(TaskbarStatus::Error, winrt::to_hresult());
        }
    }

    void QueueDetach() noexcept {
        CoreDispatcher dispatcher{nullptr};
        {
            std::scoped_lock lock{dispatcher_mutex_};
            dispatcher = dispatcher_;
        }
        if (!dispatcher) {
            // Attachments are only created after a dispatcher is captured. If
            // it is unavailable, never leave a stale Attached bit behind.
            SetStatus(TaskbarStatus::Detached);
            return;
        }

        AddRef();
        try {
            dispatcher.RunAsync(
                CoreDispatcherPriority::Normal,
                [this]() noexcept {
                    bool retry = false;
                    try {
                        std::scoped_lock lock{mutex_};
                        if (!attachments_.empty()) {
                            SetStatus(TaskbarStatus::Detaching);
                        }
                        DetachAll(false);
                        retry = !attachments_.empty();
                        SetStatus(HasVisibleMarketRoot()
                                      ? TaskbarStatus::Attached
                                      : TaskbarStatus::Detached);
                    } catch (...) {
                        // The taskbar can disappear while Explorer is shutting
                        // down. Error restores the fallback tray icon and lets
                        // the host recover if this Explorer remains alive.
                        SetStatus(TaskbarStatus::Error, winrt::to_hresult());
                    }
                    if (retry &&
                        detach_attempts_.fetch_add(
                            1, std::memory_order_acq_rel) < 3) {
                        QueueDetach();
                    }
                    Release();
                });
        } catch (...) {
            Release();
            SetStatus(TaskbarStatus::Error, winrt::to_hresult());
        }
    }

    bool HasVisibleMarketRoot() noexcept {
        for (auto const& candidate : candidates_) {
            try {
                if (auto grid = candidate.grid.get();
                    grid && FindNamedElement(grid, kMarketRootName)) {
                    return true;
                }
            } catch (...) {
                // A different candidate can still contain the live root.
            }
        }
        return false;
    }

    bool RememberCandidate(VisualElement const& visual) {
        bool name_matches = visual.Name != nullptr &&
            std::wcscmp(visual.Name, kTaskbarGridName) == 0;
        bool type_matches = visual.Type != nullptr &&
            std::wstring_view{visual.Type}.find(kTaskbarGridName) !=
                std::wstring_view::npos;
        if (!name_matches && !type_matches) {
            return false;
        }

        if (std::any_of(
                candidates_.begin(), candidates_.end(),
                [handle = visual.Handle](Candidate const& candidate) {
                    return candidate.handle == handle;
                })) {
            return false;
        }

#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugLine(L"candidate\tmatched");
#endif
        winrt::com_ptr<::IInspectable> inspectable;
        winrt::check_hresult(diagnostics_->GetIInspectableFromHandle(
            visual.Handle, inspectable.put()));
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugLine(L"candidate\tinspectable");
#endif
        auto object = inspectable.as<IInspectable>();
        auto grid = object.try_as<Grid>();
        if (grid && IsNamed(grid, kTaskbarGridName)) {
            candidates_.push_back(Candidate{visual.Handle, winrt::make_weak(grid)});
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"candidate\tstored");
#endif
            SetDispatcher(grid.Dispatcher());
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"candidate\tdispatcher");
#endif
            return true;
        }
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugLine(L"candidate\trejected");
#endif
        return false;
    }

    void ForgetHandle(InstanceHandle handle) {
        if (handle == 0) {
            return;
        }
        std::erase_if(candidates_, [handle](Candidate const& candidate) {
            return candidate.handle == handle;
        });
        std::erase_if(attachments_, [handle](Attachment const& attachment) {
            return attachment.parent_handle == handle;
        });
    }

    bool TryAttachAll(MarketSnapshot const& snapshot) {
        std::erase_if(candidates_, [](Candidate const& candidate) {
            return !candidate.grid.get();
        });
        // Explorer can rebuild a taskbar grid without delivering a matching
        // diagnostics removal callback. A strong reference to our old root
        // then remains valid even though it is no longer in Children(). Clean
        // those stale records before deciding that the market is visible.
        std::erase_if(attachments_, [](Attachment& attachment) {
            if (IsAttachmentLive(attachment)) {
                return false;
            }
            return TryDetach(attachment);
        });
        attachments_.reserve(attachments_.size() + candidates_.size());
        bool market_present = std::any_of(
            attachments_.begin(), attachments_.end(), IsAttachmentLive);

        for (auto const& candidate : candidates_) {
            auto const existing = std::find_if(
                attachments_.begin(), attachments_.end(),
                [handle = candidate.handle](Attachment const& attachment) {
                    return attachment.parent_handle == handle;
                });
            if (existing != attachments_.end()) {
                // If cleanup hit a transient XAML failure, leave the stale
                // record for the next refresh but never hide the fallback tray
                // icon on its behalf.
                market_present = IsAttachmentLive(*existing) || market_present;
                continue;
            }

            if (auto grid = candidate.grid.get()) {
                // Compatible content-hashed builds can leave another v6 watcher
                // loaded in Explorer. Reuse only this protocol version's root;
                // older roots have versioned names and detach with their owner.
                if (FindNamedElement(grid, kMarketRootName)) {
                    market_present = true;
                    continue;
                }
                market_present = TryAttach(candidate.handle, grid, snapshot) ||
                    market_present;
            }
        }
        return market_present;
    }

    static bool IsAttachmentLive(Attachment const& attachment) noexcept {
        try {
            auto parent = attachment.parent.get();
            if (!parent || !attachment.root) {
                return false;
            }
            std::uint32_t root_index = 0;
            return parent.Children().IndexOf(attachment.root, root_index);
        } catch (...) {
            // A taskbar rebuild can invalidate the collection between calls.
            // Treat an unverifiable root as absent so the host keeps its normal
            // tray icon while a later dispatcher pass retries cleanup.
            return false;
        }
    }

    bool TryAttach(
        InstanceHandle handle,
        Grid const& grid,
        MarketSnapshot const& snapshot) {
        auto clock = FindClock(grid);
        if (!clock) {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"attach\tclock-not-found");
#endif
            return false;
        }
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugLine(L"attach\tclock-found");
#endif

        auto anchor = FindDirectClockChild(grid);
        auto anchor_element = anchor.try_as<FrameworkElement>();
        if (!anchor_element) {
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
            DebugLine(L"attach\tanchor-not-found");
#endif
            return false;
        }
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugLine(L"attach\tanchor-found");
#endif

        auto definitions = grid.ColumnDefinitions();
        const auto definition_count = definitions.Size();
        const int anchor_column = Grid::GetColumn(anchor_element);
        const std::uint32_t insertion_index = std::min<std::uint32_t>(
            anchor_column < 0 ? 0u : static_cast<std::uint32_t>(anchor_column),
            definition_count);

        auto column = ColumnDefinition{};
        column.Width(GridLengthHelper::Auto());

        Attachment attachment;
        attachment.parent_handle = handle;
        attachment.parent = winrt::make_weak(grid);
        attachment.column = column;
        attachment.root = Grid{};
        attachment.symbol = TextBlock{};
        attachment.price = TextBlock{};
        attachment.change = TextBlock{};
        attachment.price_row = Grid{};

        ConfigureRoot(attachment.root);
        ConfigureText(attachment.symbol, 10.0);
        ConfigureText(attachment.price, 11.0);
        ConfigureText(attachment.change, 11.0);
        ConfigurePriceRow(attachment);
        Grid::SetRow(attachment.symbol, 0);
        Grid::SetRow(attachment.price_row, 1);
        attachment.root.Children().Append(attachment.symbol);
        attachment.root.Children().Append(attachment.price_row);

        if (auto clock_control = clock.try_as<Control>()) {
            attachment.inherited_foreground = clock_control.Foreground();
            if (attachment.inherited_foreground) {
                attachment.symbol.Foreground(attachment.inherited_foreground);
                attachment.price.Foreground(attachment.inherited_foreground);
                attachment.change.Foreground(attachment.inherited_foreground);
            }
        }

        attachment.tapped_token = attachment.root.Tapped(
            [](IInspectable const&, TappedRoutedEventArgs const&) noexcept {
                RecordAction();
            });
        attachment.right_tapped_token = attachment.root.RightTapped(
            [](IInspectable const&,
               RightTappedRoutedEventArgs const& args) noexcept {
                if (!ShowHostTrayContextMenu()) {
                    return;
                }
                try {
                    args.Handled(true);
                } catch (...) {
                    // The taskbar can rebuild the element during input dispatch.
                }
            });
        Grid::SetColumn(attachment.root, static_cast<int>(insertion_index));
        UpdateAttachment(attachment, snapshot);

        auto children = grid.Children();
        std::vector<std::pair<FrameworkElement, int>> shifted_children;
        shifted_children.reserve(children.Size());
        bool column_inserted = false;
        bool root_inserted = false;
        MutationScope mutation;
        try {
            definitions.InsertAt(insertion_index, column);
            column_inserted = true;
            for (auto const& child : children) {
                auto child_element = child.try_as<FrameworkElement>();
                if (!child_element) {
                    continue;
                }
                const int child_column = Grid::GetColumn(child_element);
                if (child_column >= static_cast<int>(insertion_index)) {
                    Grid::SetColumn(child_element, child_column + 1);
                    shifted_children.emplace_back(child_element, child_column);
                }
            }
            children.Append(attachment.root);
            root_inserted = true;
            // TryAttachAll reserved enough capacity before entering the XAML
            // mutation, so committing the attachment cannot allocate here.
            attachments_.push_back(std::move(attachment));
        } catch (...) {
            const HRESULT original_error = winrt::to_hresult();
            try {
                if (root_inserted) {
                    std::uint32_t root_index = 0;
                    if (children.IndexOf(attachment.root, root_index)) {
                        children.RemoveAt(root_index);
                    }
                }
                for (auto const& [element, original_column] : shifted_children) {
                    Grid::SetColumn(element, original_column);
                }
                if (column_inserted) {
                    std::uint32_t column_index = 0;
                    if (definitions.IndexOf(column, column_index)) {
                        definitions.RemoveAt(column_index);
                    }
                }
            } catch (...) {
                // Preserve the original failure. The taskbar may have rebuilt
                // this grid while the rollback was in progress.
            }
            winrt::throw_hresult(original_error);
        }
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        DebugLine(L"attach\tcomplete");
#endif
        return true;
    }

    static void ConfigureRoot(Grid const& root) {
        root.Name(kMarketRootName);
        root.MinWidth(76.0);
        root.MaxWidth(200.0);
        root.Margin(Thickness{4.0, 0.0, 4.0, 0.0});
        root.HorizontalAlignment(HorizontalAlignment::Center);
        root.VerticalAlignment(VerticalAlignment::Center);
        root.FlowDirection(FlowDirection::LeftToRight);
        root.IsRightTapEnabled(true);

        Color transparent{};
        transparent.A = 0;
        root.Background(SolidColorBrush{transparent});
        root.RowDefinitions().Append(RowDefinition{});
        root.RowDefinitions().Append(RowDefinition{});
        AutomationProperties::SetAutomationId(root, kMarketRootName);
    }

    static void ConfigureText(TextBlock const& text, double font_size) {
        text.FontSize(font_size);
        text.HorizontalAlignment(HorizontalAlignment::Stretch);
        text.VerticalAlignment(VerticalAlignment::Center);
        text.TextAlignment(TextAlignment::Center);
        text.TextWrapping(TextWrapping::NoWrap);
        text.TextTrimming(TextTrimming::CharacterEllipsis);
        text.MaxWidth(196.0);
    }

    static void ConfigurePriceRow(Attachment& attachment) {
        auto price_column = ColumnDefinition{};
        price_column.Width(GridLengthHelper::Auto());
        auto change_column = ColumnDefinition{};
        change_column.Width(GridLengthHelper::Auto());
        attachment.price_row.ColumnDefinitions().Append(price_column);
        attachment.price_row.ColumnDefinitions().Append(change_column);
        attachment.price_row.HorizontalAlignment(HorizontalAlignment::Center);
        attachment.price_row.VerticalAlignment(VerticalAlignment::Center);
        attachment.price_row.MaxWidth(196.0);

        attachment.price.MaxWidth(112.0);
        attachment.price.HorizontalAlignment(HorizontalAlignment::Right);
        attachment.price.TextAlignment(TextAlignment::Right);
        attachment.change.Margin(Thickness{4.0, 0.0, 0.0, 0.0});
        attachment.change.HorizontalAlignment(HorizontalAlignment::Left);
        attachment.change.TextAlignment(TextAlignment::Left);
        attachment.change.TextTrimming(TextTrimming::None);
        Grid::SetColumn(attachment.price, 0);
        Grid::SetColumn(attachment.change, 1);
        attachment.price_row.Children().Append(attachment.price);
        attachment.price_row.Children().Append(attachment.change);
    }

    static void UpdateAttachment(
        Attachment& attachment, MarketSnapshot const& snapshot) {
        attachment.symbol.Text(winrt::hstring{snapshot.symbol});
        const auto separator = snapshot.price.rfind(L' ');
        const bool has_change = separator != std::wstring::npos &&
            !snapshot.price.empty() && snapshot.price.back() == L'%';
        const std::wstring price = has_change
            ? snapshot.price.substr(0, separator)
            : snapshot.price;
        const std::wstring change = has_change
            ? snapshot.price.substr(separator + 1)
            : std::wstring{};
        attachment.price.Text(winrt::hstring{price});
        attachment.change.Text(winrt::hstring{change});
#ifdef CRYPTO_HUD_TASKBAR_DEBUG
        wchar_t accent[16]{};
        _snwprintf_s(accent, _TRUNCATE, L"\t0x%08X", snapshot.accent_argb);
        DebugLine(std::wstring{L"frame\t"} + snapshot.symbol + L"\t" +
                  snapshot.price + accent);
#endif

        std::wstring accessible_name = snapshot.symbol;
        if (!snapshot.price.empty()) {
            if (!accessible_name.empty()) {
                accessible_name.push_back(L' ');
            }
            accessible_name.append(snapshot.price);
        }
        AutomationProperties::SetName(
            attachment.root, winrt::hstring{accessible_name});

        const std::wstring tooltip = snapshot.tooltip.empty()
            ? accessible_name
            : snapshot.tooltip;
        ToolTipService::SetToolTip(
            attachment.root, winrt::box_value(winrt::hstring{tooltip}));

        if (snapshot.accent_argb == 0) {
            if (attachment.inherited_foreground) {
                attachment.change.Foreground(attachment.inherited_foreground);
            } else {
                attachment.change.ClearValue(TextBlock::ForegroundProperty());
            }
            return;
        }

        Color color{};
        color.A = static_cast<std::uint8_t>(snapshot.accent_argb >> 24);
        color.R = static_cast<std::uint8_t>(snapshot.accent_argb >> 16);
        color.G = static_cast<std::uint8_t>(snapshot.accent_argb >> 8);
        color.B = static_cast<std::uint8_t>(snapshot.accent_argb);
        attachment.change.Foreground(SolidColorBrush{color});
    }

    void UpdateAll(MarketSnapshot const& snapshot) {
        for (auto& attachment : attachments_) {
            if (IsAttachmentLive(attachment)) {
                UpdateAttachment(attachment, snapshot);
            }
        }
    }

    static bool TryDetach(Attachment& attachment) noexcept {
        try {
            auto parent = attachment.parent.get();
            if (!parent) {
                return true;
            }

            MutationScope mutation;
            auto children = parent.Children();
            std::uint32_t root_index = 0;
            if (attachment.root && children.IndexOf(attachment.root, root_index)) {
                children.RemoveAt(root_index);
            }

            auto definitions = parent.ColumnDefinitions();
            std::uint32_t column_index = 0;
            if (!attachment.column ||
                !definitions.IndexOf(attachment.column, column_index)) {
                return true;
            }

            std::vector<std::pair<FrameworkElement, int>> shifted_children;
            shifted_children.reserve(children.Size());
            try {
                for (auto const& child : children) {
                    auto child_element = child.try_as<FrameworkElement>();
                    if (!child_element) {
                        continue;
                    }
                    const int child_column = Grid::GetColumn(child_element);
                    if (child_column > static_cast<int>(column_index)) {
                        Grid::SetColumn(child_element, child_column - 1);
                        shifted_children.emplace_back(child_element, child_column);
                    }
                }
                definitions.RemoveAt(column_index);
            } catch (...) {
                for (auto const& [element, original_column] : shifted_children) {
                    try {
                        Grid::SetColumn(element, original_column);
                    } catch (...) {
                    }
                }
                throw;
            }

            if (attachment.root) {
                try {
                    attachment.root.Tapped(attachment.tapped_token);
                } catch (...) {
                }
                try {
                    attachment.root.RightTapped(attachment.right_tapped_token);
                } catch (...) {
                }
            }
            return true;
        } catch (...) {
            // Keep the attachment so a later dispatcher pass can retry. If the
            // taskbar rebuilt the parent, the weak reference will expire and
            // the next pass will consider cleanup complete.
            return false;
        }
    }

    void DetachAll(bool report_status = true) {
        if (attachments_.empty()) {
            detach_pending_.store(false, std::memory_order_release);
            if (report_status) {
                SetStatus(TaskbarStatus::Detached);
            }
            return;
        }

        if (report_status) {
            SetStatus(TaskbarStatus::Detaching);
        }
        std::erase_if(attachments_, [](Attachment& attachment) {
            return TryDetach(attachment);
        });
        detach_pending_.store(!attachments_.empty(), std::memory_order_release);
        if (report_status) {
            SetStatus(attachments_.empty()
                          ? TaskbarStatus::Detached
                          : TaskbarStatus::Detaching);
        }
    }

    winrt::com_ptr<IXamlDiagnostics> diagnostics_;
    winrt::com_ptr<IVisualTreeService3> visual_tree_service_;
    winrt::com_ptr<IContextCallback> advise_context_;
    ULONG_PTR advise_context_token_{};
    std::recursive_mutex mutex_;
    std::mutex dispatcher_mutex_;
    CoreDispatcher dispatcher_{nullptr};
    HANDLE stop_event_{nullptr};
    HANDLE stop_request_event_{nullptr};
    HANDLE worker_ready_event_{nullptr};
    HANDLE worker_start_event_{nullptr};
    HRESULT event_error_{S_OK};
    std::atomic<HRESULT> worker_apartment_result_{E_PENDING};
    std::atomic<bool> refresh_pending_{false};
    std::atomic<bool> advised_{false};
    std::atomic<bool> stop_requested_{false};
    std::atomic<bool> stopped_{false};
    std::atomic<bool> detach_pending_{false};
    std::atomic<unsigned> detach_attempts_{0};
    std::atomic<unsigned> active_diagnostics_users_{0};
    std::atomic<std::uint32_t> last_owner_pid_{0};
    bool enabled_{};
    MarketSnapshot latest_snapshot_;
    std::vector<Candidate> candidates_;
    std::vector<Attachment> attachments_;
};

struct TaskbarTap
    : winrt::implements<TaskbarTap, IObjectWithSite, winrt::non_agile> {
    // InitializeXamlDiagnosticsEx may release this bootstrap object as soon as
    // SetSite returns. Do not Stop the watcher from this object's destructor:
    // the advised callback and update-thread self-reference own its lifetime.
    // An explicit site replacement/null transition remains the stop signal.
    HRESULT STDMETHODCALLTYPE SetSite(IUnknown* site) noexcept override {
        try {
            if (watcher_) {
                const HRESULT stop_result = watcher_->Stop();
                if (FAILED(stop_result)) {
                    return stop_result;
                }
                watcher_ = nullptr;
            }
            if (site == nullptr) {
                site_ = nullptr;
                return S_OK;
            }
            site_.copy_from(site);

            winrt::com_ptr<IXamlDiagnostics> diagnostics;
            winrt::check_hresult(site->QueryInterface(
                __uuidof(IXamlDiagnostics), diagnostics.put_void()));
            winrt::com_ptr<IVisualTreeService3> visual_tree_service;
            winrt::check_hresult(site->QueryInterface(
                __uuidof(IVisualTreeService3), visual_tree_service.put_void()));

            watcher_ = winrt::make_self<VisualTreeWatcher>(
                std::move(diagnostics), std::move(visual_tree_service));
            const HRESULT result = watcher_->Start();
            if (SUCCEEDED(result)) {
                g_xaml_initialization_succeeded.store(
                    true, std::memory_order_release);
            } else {
                watcher_ = nullptr;
            }
            return result;
        } catch (...) {
            const HRESULT error = winrt::to_hresult();
            SetStatus(TaskbarStatus::Error, error);
            return error;
        }
    }

    HRESULT STDMETHODCALLTYPE GetSite(
        REFIID interface_id, void** object) noexcept override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        return site_ ? site_->QueryInterface(interface_id, object) : E_FAIL;
    }

private:
    winrt::com_ptr<IUnknown> site_;
    winrt::com_ptr<VisualTreeWatcher> watcher_;
};

template <typename Instance>
struct ClassFactory
    : winrt::implements<ClassFactory<Instance>, IClassFactory, winrt::non_agile> {
    HRESULT STDMETHODCALLTYPE CreateInstance(
        IUnknown* outer, REFIID interface_id, void** object) noexcept override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        if (outer != nullptr) {
            return CLASS_E_NOAGGREGATION;
        }

        try {
            auto instance = winrt::make_self<Instance>();
            return instance->QueryInterface(interface_id, object);
        } catch (...) {
            return winrt::to_hresult();
        }
    }

    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) noexcept override {
        if (lock) {
            ++winrt::get_module_lock();
        } else {
            --winrt::get_module_lock();
        }
        return S_OK;
    }
};

std::wstring CurrentModulePath() {
    HMODULE module = nullptr;
    if (!GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            reinterpret_cast<LPCWSTR>(&CurrentModulePath), &module)) {
        winrt::throw_last_error();
    }

    std::vector<wchar_t> buffer(512);
    for (;;) {
        const DWORD length = GetModuleFileNameW(
            module, buffer.data(), static_cast<DWORD>(buffer.size()));
        if (length == 0) {
            winrt::throw_last_error();
        }
        if (length < buffer.size() - 1) {
            return std::wstring{buffer.data(), length};
        }
        buffer.resize(buffer.size() * 2);
    }
}

using InitializeXamlDiagnosticsExFunction = HRESULT(WINAPI*)(
    LPCWSTR, DWORD, LPCWSTR, LPCWSTR, CLSID, LPCWSTR);

HRESULT InitializeTaskbarDiagnostics() noexcept {
    if (g_xaml_initialization_succeeded.load(std::memory_order_acquire)) {
        return S_OK;
    }

    bool expected = false;
    if (!g_xaml_initialization_active.compare_exchange_strong(
            expected, true, std::memory_order_acq_rel)) {
        return HRESULT_FROM_WIN32(ERROR_BUSY);
    }

    HRESULT result = E_FAIL;
    try {
        SetStatus(TaskbarStatus::Initializing);
        HMODULE xaml = GetModuleHandleW(L"Windows.UI.Xaml.dll");
        if (xaml == nullptr) {
            xaml = LoadLibraryExW(
                L"Windows.UI.Xaml.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
        }
        if (xaml == nullptr) {
            winrt::throw_last_error();
        }

        const auto initialize = reinterpret_cast<InitializeXamlDiagnosticsExFunction>(
            GetProcAddress(xaml, "InitializeXamlDiagnosticsEx"));
        if (initialize == nullptr) {
            winrt::throw_last_error();
        }

        const std::wstring module_path = CurrentModulePath();
        result = HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
        for (unsigned connection = 1; connection <= 10000; ++connection) {
            const std::wstring endpoint =
                L"VisualDiagConnection" + std::to_wstring(connection);
            const HRESULT attempt = initialize(
                endpoint.c_str(), GetCurrentProcessId(), L"",
                module_path.c_str(), kTapClsid, L"");
            if (attempt == HRESULT_FROM_WIN32(ERROR_NOT_FOUND)) {
                continue;
            }
            result = attempt;
            break;
        }

        if (SUCCEEDED(result)) {
            g_xaml_initialization_succeeded.store(
                true, std::memory_order_release);
        } else {
            SetStatus(TaskbarStatus::Error, result);
        }
    } catch (...) {
        result = winrt::to_hresult();
        SetStatus(TaskbarStatus::Error, result);
    }

    g_xaml_initialization_active.store(false, std::memory_order_release);
    return result;
}

void HandleUpdate() noexcept {
    MarketSnapshot snapshot;
    if (!ReadSnapshot(snapshot)) {
        return;
    }

    if (!snapshot.enabled) {
        SetStatus(TaskbarStatus::Detached);
        return;
    }

    InitializeTaskbarDiagnostics();
}

}  // namespace

extern "C" std::intptr_t crypto_hud_taskbar_hook_impl(
    int code, std::uintptr_t wparam, std::intptr_t lparam) noexcept {
    try {
        if (code == HC_ACTION && lparam != 0) {
            const auto* message = reinterpret_cast<const CWPSTRUCT*>(lparam);
            static const UINT update_message =
                RegisterWindowMessageW(kUpdateMessageName);
            if (update_message != 0 && message->message == update_message) {
                HandleUpdate();
            }
        }
    } catch (...) {
        SetStatus(TaskbarStatus::Error, winrt::to_hresult());
    }
    return static_cast<std::intptr_t>(CallNextHookEx(
        nullptr, code, static_cast<WPARAM>(wparam),
        static_cast<LPARAM>(lparam)));
}

extern "C" HRESULT crypto_hud_taskbar_get_class_object_impl(
    const void* class_id,
    const void* interface_id,
    void** object) noexcept {
    if (class_id == nullptr || interface_id == nullptr || object == nullptr) {
        return E_POINTER;
    }
    *object = nullptr;

    const auto& requested_class = *static_cast<const CLSID*>(class_id);
    if (!IsEqualCLSID(requested_class, kTapClsid)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    try {
        auto factory = winrt::make_self<ClassFactory<TaskbarTap>>();
        return factory->QueryInterface(
            *static_cast<const IID*>(interface_id), object);
    } catch (...) {
        return winrt::to_hresult();
    }
}

extern "C" HRESULT crypto_hud_taskbar_can_unload_now_impl() noexcept {
    return winrt::get_module_lock() ? S_FALSE : S_OK;
}

}  // namespace crypto_hud::taskbar
