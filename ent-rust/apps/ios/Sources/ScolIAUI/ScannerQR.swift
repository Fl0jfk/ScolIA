#if os(iOS)
import AVFoundation
import SwiftUI
import UIKit

/// Lecteur de QR code (AVFoundation, aucune dépendance externe).
struct ScannerQR: UIViewControllerRepresentable {
    let surCode: (String) -> Void

    func makeUIViewController(context: Context) -> ControleurScan {
        let c = ControleurScan()
        c.surCode = surCode
        return c
    }

    func updateUIViewController(_ uiViewController: ControleurScan, context: Context) {}
}

final class ControleurScan: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var surCode: ((String) -> Void)?
    private let session = AVCaptureSession()
    private var lu = false
    private var apercu: AVCaptureVideoPreviewLayer?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        view.accessibilityLabel = "Caméra : visez le QR code ScolIA"
        AVCaptureDevice.requestAccess(for: .video) { [weak self] accorde in
            DispatchQueue.main.async { accorde ? self?.configurer() : self?.afficher("Accès à la caméra refusé. Autorisez-le dans Réglages, ou saisissez l'adresse.") }
        }
    }

    private func afficher(_ message: String) {
        let l = UILabel()
        l.text = message
        l.textColor = .white
        l.numberOfLines = 0
        l.textAlignment = .center
        l.frame = view.bounds.insetBy(dx: 24, dy: 24)
        l.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(l)
        UIAccessibility.post(notification: .announcement, argument: message)
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        apercu?.frame = view.layer.bounds
    }

    private func configurer() {
        guard let camera = AVCaptureDevice.default(for: .video),
              let entree = try? AVCaptureDeviceInput(device: camera),
              session.canAddInput(entree) else { return afficher("Caméra indisponible sur cet appareil.") }
        session.addInput(entree)
        let sortie = AVCaptureMetadataOutput()
        guard session.canAddOutput(sortie) else { return }
        session.addOutput(sortie)
        sortie.setMetadataObjectsDelegate(self, queue: .main)
        sortie.metadataObjectTypes = [.qr]
        let couche = AVCaptureVideoPreviewLayer(session: session)
        couche.frame = view.layer.bounds
        couche.videoGravity = .resizeAspectFill
        view.layer.addSublayer(couche)
        apercu = couche
        DispatchQueue.global(qos: .userInitiated).async { [session] in session.startRunning() }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        session.stopRunning()
    }

    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput objets: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !lu, let o = objets.first as? AVMetadataMachineReadableCodeObject, let texte = o.stringValue else { return }
        lu = true
        session.stopRunning()
        surCode?(texte)
    }
}
#endif
