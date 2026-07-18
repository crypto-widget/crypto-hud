#pragma once

#include <cstddef>
#include <cstdint>

namespace crypto_hud::taskbar {

inline constexpr std::uint32_t kProtocolMagic = 0x4D544843;
inline constexpr std::uint16_t kProtocolVersion = 6;

inline constexpr wchar_t kMappingName[] =
    L"Local\\CryptoHud.TaskbarMarket.v6";
inline constexpr wchar_t kUpdateEventName[] =
    L"Local\\CryptoHud.TaskbarMarket.Update.v6";
inline constexpr wchar_t kActionEventName[] =
    L"Local\\CryptoHud.TaskbarMarket.Action.v6";
inline constexpr wchar_t kUpdateMessageName[] =
    L"CryptoHud.TaskbarMarket.Update.v6";

enum class TaskbarStatus : std::uint32_t {
    Disabled = 0,
    Initializing = 1,
    WaitingForVisualTree = 2,
    Attached = 3,
    Detaching = 4,
    Detached = 5,
    Error = 6,
};

struct alignas(8) SharedMarketState {
    std::uint32_t magic;
    std::uint16_t version;
    std::uint16_t size;
    volatile long sequence;
    std::uint32_t owner_pid;
    std::uint64_t heartbeat_ms;
    std::uint32_t enabled;
    volatile long status;
    volatile long error_code;
    volatile long explorer_pid;
    wchar_t symbol[64];
    wchar_t price[64];
    wchar_t tooltip[192];
    std::uint32_t accent_argb;
    volatile long action_sequence;
};

static_assert(sizeof(wchar_t) == 2);
static_assert(sizeof(long) == 4);
static_assert(alignof(SharedMarketState) == 8);
static_assert(sizeof(SharedMarketState) == 688);
static_assert(offsetof(SharedMarketState, magic) == 0);
static_assert(offsetof(SharedMarketState, version) == 4);
static_assert(offsetof(SharedMarketState, size) == 6);
static_assert(offsetof(SharedMarketState, sequence) == 8);
static_assert(offsetof(SharedMarketState, owner_pid) == 12);
static_assert(offsetof(SharedMarketState, heartbeat_ms) == 16);
static_assert(offsetof(SharedMarketState, enabled) == 24);
static_assert(offsetof(SharedMarketState, status) == 28);
static_assert(offsetof(SharedMarketState, error_code) == 32);
static_assert(offsetof(SharedMarketState, explorer_pid) == 36);
static_assert(offsetof(SharedMarketState, symbol) == 40);
static_assert(offsetof(SharedMarketState, price) == 168);
static_assert(offsetof(SharedMarketState, tooltip) == 296);
static_assert(offsetof(SharedMarketState, accent_argb) == 680);
static_assert(offsetof(SharedMarketState, action_sequence) == 684);

}  // namespace crypto_hud::taskbar
