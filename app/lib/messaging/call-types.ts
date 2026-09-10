/** Types signalisation visioconférence messagerie (mesh WebRTC). */

export const MESSAGING_CALL_MAX_PEERS = 6;

export type MessagingCallSignalAction =
  | "invite"
  | "accept"
  | "reject"
  | "join"
  | "offer"
  | "answer"
  | "ice"
  | "leave"
  | "end";

export type MessagingCallSseType =
  | "call_invite"
  | "call_accept"
  | "call_reject"
  | "call_join"
  | "call_offer"
  | "call_answer"
  | "call_ice"
  | "call_leave"
  | "call_end";

export type MessagingCallPeerInfo = {
  userId: string;
  name: string;
};

export type MessagingCallSignalPayload = {
  conversationId: string;
  callId: string;
  fromUserId: string;
  fromName: string;
  toUserId?: string;
  title?: string;
  kind?: "dm" | "group";
  participants?: MessagingCallPeerInfo[];
  sdp?: RTCSessionDescriptionInit;
  candidate?: RTCIceCandidateInit;
  reason?: string;
};

export type MessagingCallIceServer = {
  urls: string | string[];
  username?: string;
  credential?: string;
};
