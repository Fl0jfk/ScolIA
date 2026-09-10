/** Types partagés client / serveur — messagerie 1:1 + groupes. */

export type MessagingMessageType =
  | "text"
  | "image"
  | "file"
  | "audio"
  | "video"
  | "system";

export type MessagingConversationKind = "dm" | "group";

export type MessagingPeer = {
  id: string;
  name: string;
  email: string;
  imageUrl: string | null;
};

export type MessagingAttachmentDto = {
  id: string;
  s3Key: string;
  mime: string;
  size: number;
  fileName: string;
  width: number | null;
  height: number | null;
  url: string | null;
};

export type MessagingReactionDto = {
  emoji: string;
  userId: string;
  count?: number;
};

export type MessagingMessageDto = {
  id: string;
  conversationId: string;
  senderId: string;
  type: MessagingMessageType;
  body: string | null;
  replyToId: string | null;
  replyPreview: string | null;
  forwardedFromId: string | null;
  editedAt: string | null;
  deletedAt: string | null;
  createdAt: string;
  attachments: MessagingAttachmentDto[];
  reactions: MessagingReactionDto[];
};

export type MessagingConversationDto = {
  id: string;
  kind: MessagingConversationKind;
  /** Nom affiché (peer name ou titre du groupe). */
  title: string;
  peer: MessagingPeer | null;
  membersPreview: MessagingPeer[];
  memberCount: number;
  lastMessage: {
    id: string;
    body: string | null;
    type: MessagingMessageType;
    senderId: string;
    createdAt: string;
  } | null;
  unreadCount: number;
  updatedAt: string;
  lastMessageAt: string | null;
};

export type MessagingSseEventType =
  | "message"
  | "read"
  | "typing"
  | "reaction"
  | "conversation_updated"
  | "heartbeat";

export type MessagingSseEvent = {
  type: MessagingSseEventType;
  conversationId?: string;
  payload?: unknown;
};

export type MessagingEvent = MessagingSseEvent;

export type MessagingUserDto = {
  id: string;
  name: string;
  email: string;
  imageUrl: string | null;
  roles: string[];
};

export type MessagingSendAttachmentInput = {
  s3Key: string;
  mime: string;
  size: number;
  fileName: string;
  width?: number | null;
  height?: number | null;
};
