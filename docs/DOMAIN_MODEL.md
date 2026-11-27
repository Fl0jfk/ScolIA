# Modèle de domaine — principes

Le dossier élève est le point d'entrée métier, pas une table fourre-tout. Garder un modèle relationnel normalisé.

Concepts à comparer au modèle existant : Tenant/Establishment, SchoolYear/AcademicPeriod, Person, Student, Guardian, LegalResponsibility, StudentGuardian, Staff/Teacher, User/Account/Role/Permission/Membership, Class/Group/Subject/TeachingAssignment, Timetable/Lesson/Room, Attendance/Lateness/Justification, Assessment/Grade/Competency/Report, Message/Conversation/Recipient/Delivery, Document/FileObject/Attachment, Invoice/Payment, Boarding/Meal/Transport/Trip, AuditEvent.

Points essentiels :
- plusieurs responsables possibles pour un élève ; ne pas déduire droits et autorité d'un simple lien familial ;
- relations évolutives et historisées ;
- changements d'année, transferts, départs, changements de classe et réinscriptions sans écraser l'historique ;
- vues consolidées qui agrègent plusieurs entités tout en respectant les permissions ;
- tenant explicite sur chaque donnée ou justification documentée ;
- isolation côté serveur pour API, exports, jobs, fichiers, recherche et IA ;
- tests d'accès inter-établissement.

Ne pas inventer de règles juridiques, scolaires ou de conservation. Séparer les règles observées, validées et à arbitrer.
