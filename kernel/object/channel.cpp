// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// IPC Channel Implementation

#include <jowokernel/channel.h>
#include <jowokernel/types.h>

namespace object {

// Maximum messages in a channel queue
constexpr int MAX_CHANNEL_MESSAGES = 256;

// A ChannelEndpoint is one side of a bidirectional IPC connection.
class ChannelEndpoint {
public:
    ChannelEndpoint() : peer_(nullptr), message_count_(0) {}
    ~ChannelEndpoint() {
        if (peer_) {
            peer_->OnPeerClosed();
        }
    }

    void SetPeer(ChannelEndpoint* peer) {
        peer_ = peer;
    }

    // Write a message to the other end's queue
    bool Write(const ChannelMessage& msg) {
        if (!peer_) return false; // Peer is closed
        return peer_->Enqueue(msg);
    }

    // Read a message from this end's queue
    bool Read(ChannelMessage* out_msg) {
        if (message_count_ == 0) return false;
        
        // Simple queue pop (implementation simplified)
        *out_msg = queue_[0];
        for (int i = 1; i < message_count_; i++) {
            queue_[i - 1] = queue_[i];
        }
        message_count_--;
        return true;
    }

    void OnPeerClosed() {
        peer_ = nullptr;
    }

private:
    bool Enqueue(const ChannelMessage& msg) {
        if (message_count_ >= MAX_CHANNEL_MESSAGES) return false;
        
        // In reality, msg payload and handles must be deep-copied into kernel space
        queue_[message_count_++] = msg;
        return true;
    }

    ChannelEndpoint* peer_;
    ChannelMessage queue_[MAX_CHANNEL_MESSAGES];
    int message_count_;
};

// Factory to create a connected pair of channel endpoints
class ChannelDispatcher {
public:
    static bool Create(ChannelEndpoint** out_ep1, ChannelEndpoint** out_ep2) {
        *out_ep1 = new ChannelEndpoint();
        *out_ep2 = new ChannelEndpoint();
        
        (*out_ep1)->SetPeer(*out_ep2);
        (*out_ep2)->SetPeer(*out_ep1);
        
        return true;
    }
};

} // namespace object
